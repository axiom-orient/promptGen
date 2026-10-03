#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::env;
use std::ffi::OsString;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

use promptgen_codex::{
    CodexRunConfig, LunaReviewConfig, execute_image_generation, review_image_prompt,
};
use promptgen_core::catalog::{CatalogError, catalog, catalog_json};
use promptgen_core::image::{ImagePromptRequest, available_lut_presets};
use promptgen_core::json::{JsonValue, parse};
use promptgen_core::{
    CompilationOutcome, CompilationStatus, InterviewRequest, InterviewStatus, compilation_schema,
    compile_image_prompt, image_schema, interview_schema, run_interview,
};
use promptgen_mcp::{
    MAX_BODY_BYTES, McpHttpConfig, serve_http as serve_mcp_http, serve_stdio as serve_mcp_stdio,
};
use promptgen_web::{WebConfig, serve as serve_web};

const EXIT_OK: u8 = 0;
const EXIT_USAGE_OR_IO: u8 = 2;
const EXIT_VALIDATION: u8 = 3;
const EXIT_EXECUTION: u8 = 4;
const MAX_INPUT_BYTES: usize = 16 * 1024 * 1024;
static NEXT_NONCE: AtomicU64 = AtomicU64::new(1);

fn main() -> ExitCode {
    match dispatch(env::args_os().skip(1).collect()) {
        Ok(code) => ExitCode::from(code),
        Err(error) if error.exit_code == EXIT_OK => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error[{}]: {}", error.code, error.message);
            ExitCode::from(error.exit_code)
        }
    }
}

fn dispatch(arguments: Vec<OsString>) -> Result<u8, CliError> {
    let mut cursor = ArgCursor::new(arguments);
    let Some(command) = cursor.next_string()? else {
        print_help();
        return Ok(EXIT_OK);
    };
    match command.as_str() {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(EXIT_OK)
        }
        "version" | "--version" | "-V" => {
            println!("promptgen {}", env!("CARGO_PKG_VERSION"));
            Ok(EXIT_OK)
        }
        "screen" => compile_screen(parse_compile_options(cursor, print_screen_help)?),
        "image" => compile_image(parse_image_options(cursor)?),
        "interview" => run_guided_interview(parse_compile_options(cursor, print_interview_help)?),
        "serve" => serve_ui(parse_serve_options(cursor)?),
        "mcp" => serve_mcp(parse_mcp_options(cursor)?),
        "schema" => emit_schema(parse_schema_options(cursor)?),
        "catalog" => emit_catalog(parse_catalog_options(cursor)?),
        "lut-presets" => emit_lut_presets(parse_lut_options(cursor)?),
        other => Err(CliError::usage(format!("unknown command {other:?}"))),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFormat {
    Json,
    Text,
}

impl OutputFormat {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "json" => Ok(Self::Json),
            "text" => Ok(Self::Text),
            _ => Err(CliError::usage(format!(
                "invalid --format {value:?}; expected json or text"
            ))),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ImageMode {
    PromptOnly,
    LunaRefine,
    CodexImagegen,
}

impl ImageMode {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "prompt-only" => Ok(Self::PromptOnly),
            "luna-refine" => Ok(Self::LunaRefine),
            "codex-imagegen" => Ok(Self::CodexImagegen),
            _ => Err(CliError::usage(format!(
                "invalid --mode {value:?}; expected prompt-only, luna-refine, codex-imagegen"
            ))),
        }
    }
}

#[derive(Debug)]
struct CompileOptions {
    input: PathBuf,
    format: OutputFormat,
    result: Option<PathBuf>,
    force: bool,
}

#[derive(Debug)]
struct ImageOptions {
    compile: CompileOptions,
    mode: ImageMode,
    image_output: Option<PathBuf>,
    reference_image: Option<PathBuf>,
    codex_binary: PathBuf,
    codex_home: Option<PathBuf>,
    timeout: Duration,
    artifact_wait: Duration,
}

#[derive(Debug)]
struct SchemaOptions {
    target: String,
    output: OutputOptions,
}

#[derive(Debug)]
struct CatalogOptions {
    format: OutputFormat,
    output: OutputOptions,
}

#[derive(Debug)]
struct OutputOptions {
    result: Option<PathBuf>,
    force: bool,
}

#[derive(Debug)]
struct ServeOptions {
    config: WebConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum McpTransport {
    Stdio,
    Http,
}

impl McpTransport {
    fn parse(value: &str) -> Result<Self, CliError> {
        match value {
            "stdio" => Ok(Self::Stdio),
            "http" => Ok(Self::Http),
            _ => Err(CliError::usage(format!(
                "invalid --transport {value:?}; expected stdio or http"
            ))),
        }
    }
}

#[derive(Debug)]
struct McpOptions {
    transport: McpTransport,
    config: McpHttpConfig,
}

fn parse_compile_options(
    mut cursor: ArgCursor,
    print_help: fn(),
) -> Result<CompileOptions, CliError> {
    let mut input = PathBuf::from("-");
    let mut format = OutputFormat::Json;
    let mut result = None;
    let mut force = false;
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        match argument.as_str() {
            "--input" => {
                claim_option(&mut seen, "--input")?;
                input = PathBuf::from(cursor.required_value("--input")?);
            }
            "--format" => {
                claim_option(&mut seen, "--format")?;
                format = OutputFormat::parse(&cursor.required_value("--format")?)?;
            }
            "--result" => {
                claim_option(&mut seen, "--result")?;
                result = Some(PathBuf::from(cursor.required_value(&argument)?));
            }
            "--force" => {
                claim_option(&mut seen, "--force")?;
                force = true;
            }
            "--help" | "-h" => {
                print_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    Ok(CompileOptions {
        input,
        format,
        result,
        force,
    })
}

fn parse_image_options(mut cursor: ArgCursor) -> Result<ImageOptions, CliError> {
    let mut compile = CompileOptions {
        input: PathBuf::from("-"),
        format: OutputFormat::Json,
        result: None,
        force: false,
    };
    let mut mode = ImageMode::PromptOnly;
    let mut image_output = None;
    let mut reference_image = None;
    let mut codex_binary = PathBuf::from("codex");
    let mut codex_home = None;
    let mut timeout = Duration::from_secs(240);
    let mut artifact_wait = Duration::from_secs(30);
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        match argument.as_str() {
            "--input" => {
                claim_option(&mut seen, "--input")?;
                compile.input = PathBuf::from(cursor.required_value("--input")?);
            }
            "--format" => {
                claim_option(&mut seen, "--format")?;
                compile.format = OutputFormat::parse(&cursor.required_value("--format")?)?;
            }
            "--result" => {
                claim_option(&mut seen, "--result")?;
                compile.result = Some(PathBuf::from(cursor.required_value(&argument)?));
            }
            "--force" => {
                claim_option(&mut seen, "--force")?;
                compile.force = true;
            }
            "--mode" => {
                claim_option(&mut seen, "--mode")?;
                mode = ImageMode::parse(&cursor.required_value("--mode")?)?;
            }
            "--image-output" => {
                claim_option(&mut seen, "--image-output")?;
                image_output = Some(PathBuf::from(cursor.required_value("--image-output")?));
            }
            "--reference-image" => {
                claim_option(&mut seen, "--reference-image")?;
                reference_image = Some(PathBuf::from(cursor.required_value("--reference-image")?));
            }
            "--codex-binary" => {
                claim_option(&mut seen, "--codex-binary")?;
                codex_binary = PathBuf::from(cursor.required_value("--codex-binary")?);
            }
            "--codex-home" => {
                claim_option(&mut seen, "--codex-home")?;
                codex_home = Some(PathBuf::from(cursor.required_value("--codex-home")?));
            }
            "--timeout-seconds" => {
                claim_option(&mut seen, "--timeout-seconds")?;
                timeout = Duration::from_secs(parse_seconds(
                    &cursor.required_value("--timeout-seconds")?,
                    "--timeout-seconds",
                    3_600,
                )?);
            }
            "--artifact-wait-seconds" => {
                claim_option(&mut seen, "--artifact-wait-seconds")?;
                artifact_wait = Duration::from_secs(parse_seconds(
                    &cursor.required_value("--artifact-wait-seconds")?,
                    "--artifact-wait-seconds",
                    600,
                )?);
            }
            "--help" | "-h" => {
                print_image_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    Ok(ImageOptions {
        compile,
        mode,
        image_output,
        reference_image,
        codex_binary,
        codex_home,
        timeout,
        artifact_wait,
    })
}

fn parse_schema_options(mut cursor: ArgCursor) -> Result<SchemaOptions, CliError> {
    let target = match cursor.next_string()? {
        Some(value) if matches!(value.as_str(), "--help" | "-h") => {
            print_schema_help();
            return Err(CliError::printed_help());
        }
        Some(value) => value,
        None => return Err(CliError::usage("schema requires a target")),
    };
    let mut output = OutputOptions {
        result: None,
        force: false,
    };
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        match argument.as_str() {
            "--result" => {
                claim_option(&mut seen, "--result")?;
                output.result = Some(PathBuf::from(cursor.required_value(&argument)?));
            }
            "--force" => {
                claim_option(&mut seen, "--force")?;
                output.force = true;
            }
            "--help" | "-h" => {
                print_schema_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    Ok(SchemaOptions { target, output })
}

fn parse_catalog_options(mut cursor: ArgCursor) -> Result<CatalogOptions, CliError> {
    let mut format = OutputFormat::Json;
    let mut output = OutputOptions {
        result: None,
        force: false,
    };
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        match argument.as_str() {
            "--format" => {
                claim_option(&mut seen, "--format")?;
                format = OutputFormat::parse(&cursor.required_value("--format")?)?;
            }
            "--result" => {
                claim_option(&mut seen, "--result")?;
                output.result = Some(PathBuf::from(cursor.required_value(&argument)?));
            }
            "--force" => {
                claim_option(&mut seen, "--force")?;
                output.force = true;
            }
            "--help" | "-h" => {
                print_catalog_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    Ok(CatalogOptions { format, output })
}

fn parse_lut_options(mut cursor: ArgCursor) -> Result<OutputOptions, CliError> {
    let mut result = None;
    let mut force = false;
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        match argument.as_str() {
            "--result" => {
                claim_option(&mut seen, "--result")?;
                result = Some(PathBuf::from(cursor.required_value(&argument)?));
            }
            "--force" => {
                claim_option(&mut seen, "--force")?;
                force = true;
            }
            "--help" | "-h" => {
                print_lut_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    Ok(OutputOptions { result, force })
}

fn parse_serve_options(mut cursor: ArgCursor) -> Result<ServeOptions, CliError> {
    let mut config = WebConfig::default();
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        match argument.as_str() {
            "--bind" => {
                claim_option(&mut seen, "--bind")?;
                let value = cursor.required_value("--bind")?;
                config.bind = value.parse::<SocketAddr>().map_err(|error| {
                    CliError::usage(format!("invalid --bind {value:?}: {error}"))
                })?;
            }
            "--output-dir" => {
                claim_option(&mut seen, "--output-dir")?;
                config.output_dir = PathBuf::from(cursor.required_value("--output-dir")?);
            }
            "--catalog-dir" => {
                claim_option(&mut seen, "--catalog-dir")?;
                config.catalog_dir = Some(PathBuf::from(cursor.required_value("--catalog-dir")?));
            }
            "--codex-binary" => {
                claim_option(&mut seen, "--codex-binary")?;
                config.codex_binary = PathBuf::from(cursor.required_value("--codex-binary")?);
            }
            "--codex-home" => {
                claim_option(&mut seen, "--codex-home")?;
                config.codex_home = PathBuf::from(cursor.required_value("--codex-home")?);
            }
            "--timeout-seconds" => {
                claim_option(&mut seen, "--timeout-seconds")?;
                config.request_timeout = Duration::from_secs(parse_seconds(
                    &cursor.required_value("--timeout-seconds")?,
                    "--timeout-seconds",
                    3_600,
                )?);
            }
            "--artifact-wait-seconds" => {
                claim_option(&mut seen, "--artifact-wait-seconds")?;
                config.artifact_wait_timeout = Duration::from_secs(parse_seconds(
                    &cursor.required_value("--artifact-wait-seconds")?,
                    "--artifact-wait-seconds",
                    600,
                )?);
            }
            "--help" | "-h" => {
                print_serve_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    Ok(ServeOptions { config })
}

fn parse_mcp_options(mut cursor: ArgCursor) -> Result<McpOptions, CliError> {
    let mut transport = McpTransport::Stdio;
    let mut config = McpHttpConfig::default();
    let mut bearer_token = None;
    let mut http_option_seen = false;
    let mut seen = BTreeSet::new();
    while let Some(argument) = cursor.next_string()? {
        match argument.as_str() {
            "--transport" => {
                claim_option(&mut seen, "--transport")?;
                transport = McpTransport::parse(&cursor.required_value("--transport")?)?;
            }
            "--bind" => {
                claim_option(&mut seen, "--bind")?;
                http_option_seen = true;
                let value = cursor.required_value("--bind")?;
                config.bind = value.parse::<SocketAddr>().map_err(|error| {
                    CliError::usage(format!("invalid --bind {value:?}: {error}"))
                })?;
            }
            "--bearer-token" => {
                claim_option(&mut seen, "--bearer-token")?;
                http_option_seen = true;
                let value = cursor.required_value("--bearer-token")?;
                if value.is_empty() {
                    return Err(CliError::usage("--bearer-token must not be empty"));
                }
                bearer_token = Some(value);
            }
            "--allowed-host" => {
                claim_option(&mut seen, "--allowed-host")?;
                http_option_seen = true;
                let value = cursor.required_value("--allowed-host")?;
                config.allowed_hosts = value
                    .split(',')
                    .map(str::trim)
                    .filter(|host| !host.is_empty())
                    .map(ToOwned::to_owned)
                    .collect();
                if config.allowed_hosts.is_empty() {
                    return Err(CliError::usage("--allowed-host must contain a hostname"));
                }
            }
            "--max-body-bytes" => {
                claim_option(&mut seen, "--max-body-bytes")?;
                http_option_seen = true;
                config.max_body_bytes =
                    parse_body_bytes(&cursor.required_value("--max-body-bytes")?)?;
            }
            "--help" | "-h" => {
                print_mcp_help();
                return Err(CliError::printed_help());
            }
            _ => return Err(CliError::usage(format!("unknown option {argument:?}"))),
        }
    }
    if transport == McpTransport::Stdio && http_option_seen {
        return Err(CliError::usage(
            "HTTP-only MCP options require --transport http",
        ));
    }
    if bearer_token.is_none() {
        bearer_token =
            match env::var_os("PROMPTGEN_MCP_BEARER_TOKEN") {
                Some(value) => Some(value.into_string().map_err(|_| {
                    CliError::usage("PROMPTGEN_MCP_BEARER_TOKEN must be valid UTF-8")
                })?),
                None => None,
            };
    }
    config.bearer_token = bearer_token;
    Ok(McpOptions { transport, config })
}

fn parse_body_bytes(value: &str) -> Result<usize, CliError> {
    let bytes = value
        .parse::<usize>()
        .map_err(|_| CliError::usage("--max-body-bytes expects a positive integer"))?;
    if bytes == 0 || bytes > MAX_BODY_BYTES {
        return Err(CliError::usage(format!(
            "--max-body-bytes must be within 1..={MAX_BODY_BYTES}"
        )));
    }
    Ok(bytes)
}

fn run_guided_interview(options: CompileOptions) -> Result<u8, CliError> {
    let request = InterviewRequest::from_json(read_json(&options.input)?)
        .map_err(|error| CliError::validation(error.to_string()))?;
    let outcome = run_interview(&request);
    let text = match options.format {
        OutputFormat::Json => outcome.to_json().to_pretty_string(),
        OutputFormat::Text => interview_to_text(&outcome),
    };
    write_output(&text, options.result.as_deref(), options.force)?;
    Ok(if outcome.status == InterviewStatus::Invalid {
        EXIT_VALIDATION
    } else {
        EXIT_OK
    })
}

fn interview_to_text(outcome: &promptgen_core::InterviewOutcome) -> String {
    if let Some(compilation) = &outcome.compilation {
        return compilation.to_text();
    }
    let mut text = format!(
        "status={}\nkind={}\n",
        outcome.status.as_str(),
        outcome.kind.as_str()
    );
    if !outcome.inferences.is_empty() {
        text.push_str("\n[inferences]\n");
        for inference in &outcome.inferences {
            text.push_str("- ");
            text.push_str(inference);
            text.push('\n');
        }
    }
    if !outcome.questions.is_empty() {
        text.push_str("\n[questions]\n");
        for question in &outcome.questions {
            text.push_str(&format!(
                "- {}: {} ({})\n  {}\n",
                question.id, question.label, question.control, question.help
            ));
        }
    }
    text
}

fn serve_ui(options: ServeOptions) -> Result<u8, CliError> {
    serve_web(options.config).map_err(|error| CliError::execution(error.to_string()))?;
    Ok(EXIT_OK)
}

fn serve_mcp(options: McpOptions) -> Result<u8, CliError> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| CliError::execution(format!("create MCP runtime: {error}")))?;
    match options.transport {
        McpTransport::Stdio => runtime
            .block_on(serve_mcp_stdio())
            .map_err(CliError::execution)?,
        McpTransport::Http => runtime
            .block_on(serve_mcp_http(options.config))
            .map_err(|error| CliError::execution(error.to_string()))?,
    }
    Ok(EXIT_OK)
}

fn compile_image(options: ImageOptions) -> Result<u8, CliError> {
    let request = ImagePromptRequest::from_json(read_json(&options.compile.input)?)
        .map_err(|error| CliError::validation(error.to_string()))?;
    if options.mode == ImageMode::CodexImagegen {
        preflight_image_paths(&options)?;
    }
    let compilation = compile_image_prompt(&request);
    if compilation.status == CompilationStatus::Invalid {
        return emit_compilation(compilation, options.compile);
    }
    match options.mode {
        ImageMode::PromptOnly => {
            if options.image_output.is_some() || options.reference_image.is_some() {
                return Err(CliError::usage(
                    "--image-output and --reference-image require an image execution mode",
                ));
            }
            emit_compilation(compilation, options.compile)
        }
        ImageMode::LunaRefine => {
            if options.image_output.is_some() || options.reference_image.is_some() {
                return Err(CliError::usage(
                    "--image-output and --reference-image are not valid with --mode luna-refine",
                ));
            }
            let codex_home = resolve_codex_home(options.codex_home)?;
            let mut config = LunaReviewConfig::new(options.codex_binary, codex_home);
            config.timeout = options.timeout;
            let refinement = review_image_prompt(&compilation, &request, &config)
                .map_err(|error| CliError::execution(error.to_string()))?;
            let value = JsonValue::object([
                ("compilation", compilation.to_json()),
                ("prompt_refinement", refinement.to_json()),
            ]);
            let text = match options.compile.format {
                OutputFormat::Json => value.to_pretty_string(),
                OutputFormat::Text => format!(
                    "{}\n\n[Luna review]\nprovider={}\nmodel={}\nsummary={}\nsource_sha256={}\nrefined_sha256={}\nadditions={}\n\n{}\n",
                    compilation.to_text().trim_end(),
                    refinement.provider(),
                    refinement.model(),
                    refinement.review_summary(),
                    refinement.source_prompt_sha256(),
                    refinement.refined_prompt_sha256(),
                    refinement.additions().len(),
                    refinement.prompt()
                ),
            };
            write_output(
                &text,
                options.compile.result.as_deref(),
                options.compile.force,
            )?;
            Ok(EXIT_OK)
        }
        ImageMode::CodexImagegen => {
            let image_output = options
                .image_output
                .ok_or_else(|| CliError::usage("--mode codex-imagegen requires --image-output"))?;
            if let Some(result) = options.compile.result.as_deref() {
                preflight_output(result, options.compile.force)?;
            }
            preflight_output(&image_output, options.compile.force)?;
            let codex_home = resolve_codex_home(options.codex_home)?;
            let mut config = CodexRunConfig::new(
                options.codex_binary.clone(),
                codex_home.clone(),
                image_output,
            );
            config.timeout = options.timeout;
            config.artifact_wait_timeout = options.artifact_wait;
            config.overwrite = options.compile.force;
            config.reference_image = options.reference_image;
            promptgen_codex::preflight_image_execution(&request, &config)
                .map_err(|error| CliError::validation(error.to_string()))?;
            let mut luna_config = LunaReviewConfig::new(options.codex_binary, codex_home);
            luna_config.timeout = options.timeout;
            let refinement = review_image_prompt(&compilation, &request, &luna_config)
                .map_err(|error| CliError::execution(error.to_string()))?;
            let receipt = execute_image_generation(&compilation, &request, &refinement, &config)
                .map_err(|error| CliError::execution(error.to_string()))?;
            let value = JsonValue::object([
                ("compilation", compilation.to_json()),
                ("execution", receipt.to_json()),
            ]);
            let text = match options.compile.format {
                OutputFormat::Json => value.to_pretty_string(),
                OutputFormat::Text => format!(
                    "{}\n\n[execution]\nbackend={}\nluna_model={}\ncompiled_prompt_sha256={}\nexecuted_prompt_sha256={}\nreview_additions={}\nthread_id={}\ncall_id={}\noutput={}\nsha256={}\nsize={}x{}\n",
                    compilation.to_text().trim_end(),
                    receipt.backend,
                    receipt.prompt_refinement.model(),
                    receipt.compiled_prompt_sha256,
                    receipt.executed_prompt_sha256,
                    receipt.prompt_refinement.additions().len(),
                    receipt.thread_id,
                    receipt.image_call_id,
                    receipt.output_artifact.display(),
                    receipt.sha256,
                    receipt.width,
                    receipt.height
                ),
            };
            if let Err(error) = write_output(
                &text,
                options.compile.result.as_deref(),
                options.compile.force,
            ) {
                return Err(CliError::execution_with_code(
                    "CLI_RECEIPT_WRITE_AFTER_IMAGE_PUBLISHED",
                    format!(
                        "image is already published at {} with sha256={}; receipt output failed: {}",
                        receipt.output_artifact.display(),
                        receipt.sha256,
                        error.message
                    ),
                ));
            }
            Ok(EXIT_OK)
        }
    }
}

fn preflight_image_paths(options: &ImageOptions) -> Result<(), CliError> {
    let image = options
        .image_output
        .as_deref()
        .ok_or_else(|| CliError::usage("--mode codex-imagegen requires --image-output"))?;
    let result = options.compile.result.as_deref();
    let reference = options.reference_image.as_deref();
    for (left, right, message) in [
        (
            Some(image),
            result,
            "--result and --image-output must resolve to different files",
        ),
        (
            Some(image),
            reference,
            "--reference-image and --image-output must resolve to different files",
        ),
        (
            result,
            reference,
            "--result and --reference-image must resolve to different files",
        ),
    ] {
        if let (Some(left), Some(right)) = (left, right)
            && resolve_paths_alias(left, right)
                .map_err(|error| CliError::io("resolve execution path alias", left, error))?
        {
            return Err(CliError::usage(message));
        }
    }
    Ok(())
}

fn emit_compilation(
    compilation: CompilationOutcome,
    options: CompileOptions,
) -> Result<u8, CliError> {
    let text = match options.format {
        OutputFormat::Json => compilation.to_json().to_pretty_string(),
        OutputFormat::Text => compilation.to_text(),
    };
    write_output(&text, options.result.as_deref(), options.force)?;
    Ok(if compilation.status == CompilationStatus::Invalid {
        EXIT_VALIDATION
    } else {
        EXIT_OK
    })
}

fn emit_schema(options: SchemaOptions) -> Result<u8, CliError> {
    let schema = match options.target.as_str() {
        "image" => image_schema(),
        "interview" => interview_schema(),
        "compilation" => compilation_schema(),
        target => {
            return Err(CliError::usage(format!(
                "unknown schema target {target:?}; expected image, interview, or compilation"
            )));
        }
    };
    write_output(
        &schema.to_pretty_string(),
        options.output.result.as_deref(),
        options.output.force,
    )?;
    Ok(EXIT_OK)
}

fn emit_catalog(options: CatalogOptions) -> Result<u8, CliError> {
    let text = match options.format {
        OutputFormat::Json => catalog_json()
            .map_err(|error| CliError::validation(format!("catalog load failed: {error}")))?
            .to_pretty_string(),
        // The text listing names four fields, so it reads the typed entries instead
        // of serializing the whole public catalog only to pick strings back out.
        OutputFormat::Text => catalog_text()
            .map_err(|error| CliError::validation(format!("catalog load failed: {error}")))?,
    };
    write_output(
        &text,
        options.output.result.as_deref(),
        options.output.force,
    )?;
    Ok(EXIT_OK)
}

fn emit_lut_presets(options: OutputOptions) -> Result<u8, CliError> {
    write_output(
        &available_lut_presets().to_pretty_string(),
        options.result.as_deref(),
        options.force,
    )?;
    Ok(EXIT_OK)
}

fn catalog_text() -> Result<String, CatalogError> {
    let mut output = String::new();
    for entry in &catalog()?.entries {
        output.push_str(&format!(
            "{}\t{}\t{}\n",
            entry.id, entry.name_ko, entry.intent
        ));
    }
    Ok(output)
}

fn read_json(path: &Path) -> Result<JsonValue, CliError> {
    let text = read_input(path)?;
    parse(&text).map_err(|error| CliError::validation(format!("JSON parse failed: {error}")))
}

fn read_input(path: &Path) -> Result<String, CliError> {
    let mut bytes = Vec::new();
    if path == Path::new("-") {
        io::stdin()
            .take((MAX_INPUT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| CliError::io("read stdin", path, error))?;
    } else {
        let metadata =
            fs::metadata(path).map_err(|error| CliError::io("stat input", path, error))?;
        if !metadata.is_file() {
            return Err(CliError::usage(format!(
                "input is not a regular file: {}",
                path.display()
            )));
        }
        if metadata.len() > MAX_INPUT_BYTES as u64 {
            return Err(CliError::usage(format!(
                "input exceeds {} bytes: {}",
                MAX_INPUT_BYTES,
                path.display()
            )));
        }
        File::open(path)
            .and_then(|file| {
                file.take((MAX_INPUT_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)
            })
            .map_err(|error| CliError::io("read input", path, error))?;
    }
    if bytes.len() > MAX_INPUT_BYTES {
        return Err(CliError::usage(format!(
            "input exceeds {MAX_INPUT_BYTES} bytes"
        )));
    }
    String::from_utf8(bytes)
        .map_err(|error| CliError::validation(format!("input is not UTF-8: {error}")))
}

fn write_output(text: &str, path: Option<&Path>, force: bool) -> Result<(), CliError> {
    let Some(path) = path else {
        let mut stdout = io::stdout().lock();
        stdout
            .write_all(text.as_bytes())
            .and_then(|()| {
                if text.ends_with('\n') {
                    Ok(())
                } else {
                    stdout.write_all(b"\n")
                }
            })
            .and_then(|()| stdout.flush())
            .map_err(|error| CliError::io("write stdout", Path::new("<stdout>"), error))?;
        return Ok(());
    };
    preflight_output(path, force)?;
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| CliError::io("create output directory", parent, error))?;
    let name = path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("result");
    let temporary = parent.join(format!(
        ".{name}.promptgen-partial-{}-{}-{}",
        std::process::id(),
        unix_millis(),
        NEXT_NONCE.fetch_add(1, Ordering::Relaxed)
    ));
    let operation = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| CliError::io("create temporary output", &temporary, error))?;
        file.write_all(text.as_bytes())
            .and_then(|()| file.sync_all())
            .map_err(|error| CliError::io("write temporary output", &temporary, error))?;
        drop(file);
        publish_output(&temporary, path, force)
    })();
    if operation.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    operation
}

fn preflight_output(path: &Path, force: bool) -> Result<(), CliError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err(CliError::usage(format!(
                    "output is not a regular file: {}",
                    path.display()
                )));
            }
            if !force {
                return Err(CliError::usage(format!(
                    "output already exists; use --force to replace it: {}",
                    path.display()
                )));
            }
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(CliError::io("inspect output", path, error)),
    }
    Ok(())
}

fn publish_output(temporary: &Path, destination: &Path, force: bool) -> Result<(), CliError> {
    if force {
        fs::rename(temporary, destination)
            .map_err(|error| CliError::io("publish replacement output", destination, error))?;
    } else {
        match fs::hard_link(temporary, destination) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                return Err(CliError::usage(format!(
                    "output already exists; use --force to replace it: {}",
                    destination.display()
                )));
            }
            Err(error) => {
                return Err(CliError::io("publish new output", destination, error));
            }
        }
        // Destination creation is the publication point. Cleanup-only failure must not turn
        // a published result into a false failure.
        let _ = fs::remove_file(temporary);
    }
    let parent = destination
        .parent()
        .filter(|value| !value.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    if let Err(error) = sync_output_parent(parent) {
        return Err(CliError::execution_with_code(
            "CLI_OUTPUT_PUBLISHED_NOT_DURABLE",
            format!(
                "output is already published at {}, but directory sync failed: {error}",
                destination.display()
            ),
        ));
    }
    Ok(())
}

fn sync_output_parent(parent: &Path) -> io::Result<()> {
    #[cfg(unix)]
    {
        File::open(parent)?.sync_all()?;
    }
    Ok(())
}

fn resolve_codex_home(explicit: Option<PathBuf>) -> Result<PathBuf, CliError> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    if let Some(path) = env::var_os("CODEX_HOME") {
        return Ok(PathBuf::from(path));
    }
    let home = env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .ok_or_else(|| CliError::usage("cannot resolve CODEX_HOME; pass --codex-home"))?;
    Ok(PathBuf::from(home).join(".codex"))
}

#[derive(Debug)]
struct CliAliasPath {
    resolved: PathBuf,
    identity: Option<CliAliasFileIdentity>,
}

#[cfg(unix)]
type CliAliasFileIdentity = (u64, u64);

#[cfg(not(unix))]
type CliAliasFileIdentity = PathBuf;

fn resolve_paths_alias(left: &Path, right: &Path) -> io::Result<bool> {
    let left_absolute = absolute_lexical(left)?;
    let right_absolute = absolute_lexical(right)?;
    if left_absolute == right_absolute {
        return Ok(true);
    }
    let left = resolve_alias_path(left_absolute)?;
    let right = resolve_alias_path(right_absolute)?;
    Ok(left.resolved == right.resolved
        || matches!((left.identity, right.identity), (Some(left), Some(right)) if left == right))
}

fn resolve_alias_path(absolute: PathBuf) -> io::Result<CliAliasPath> {
    let mut existing = absolute.clone();
    let mut missing_components = Vec::new();
    loop {
        match fs::symlink_metadata(&existing) {
            Ok(_) => break,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let component = existing.file_name().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::NotFound,
                        format!("path has no existing prefix: {}", absolute.display()),
                    )
                })?;
                missing_components.push(PathBuf::from(component));
                if !existing.pop() {
                    return Err(error);
                }
            }
            Err(error) => return Err(error),
        }
    }
    let mut resolved = fs::canonicalize(&existing)?;
    if !missing_components.is_empty() && !fs::metadata(&resolved)?.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            format!("path prefix is not a directory: {}", existing.display()),
        ));
    }
    for component in missing_components.iter().rev() {
        resolved.push(component);
    }
    let identity = match fs::symlink_metadata(&absolute) {
        Ok(_) => {
            resolved = fs::canonicalize(&absolute)?;
            alias_file_identity(&absolute)?
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => None,
        Err(error) => return Err(error),
    };
    Ok(CliAliasPath { resolved, identity })
}

fn alias_file_identity(path: &Path) -> io::Result<Option<CliAliasFileIdentity>> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Ok(None);
    }
    #[cfg(unix)]
    {
        Ok(Some((metadata.dev(), metadata.ino())))
    }
    #[cfg(not(unix))]
    {
        Ok(Some(fs::canonicalize(path)?))
    }
}

fn absolute_lexical(path: &Path) -> io::Result<PathBuf> {
    let mut output = if path.is_absolute() {
        PathBuf::new()
    } else {
        env::current_dir()?
    };
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => output.push(prefix.as_os_str()),
            std::path::Component::RootDir => {
                output.push(Path::new(std::path::MAIN_SEPARATOR_STR));
            }
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                output.pop();
            }
            std::path::Component::Normal(value) => output.push(value),
        }
    }
    Ok(output)
}

fn claim_option(seen: &mut BTreeSet<&'static str>, option: &'static str) -> Result<(), CliError> {
    if seen.insert(option) {
        Ok(())
    } else {
        Err(CliError::usage(format!(
            "option {option} was provided more than once"
        )))
    }
}

fn parse_seconds(value: &str, option: &str, maximum: u64) -> Result<u64, CliError> {
    let seconds = value
        .parse::<u64>()
        .map_err(|_| CliError::usage(format!("{option} expects a positive integer")))?;
    if seconds == 0 || seconds > maximum {
        return Err(CliError::usage(format!(
            "{option} must be within 1..={maximum} seconds"
        )));
    }
    Ok(seconds)
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

struct ArgCursor {
    values: Vec<OsString>,
    index: usize,
}

impl ArgCursor {
    const fn new(values: Vec<OsString>) -> Self {
        Self { values, index: 0 }
    }

    fn next_string(&mut self) -> Result<Option<String>, CliError> {
        let Some(value) = self.values.get(self.index) else {
            return Ok(None);
        };
        self.index += 1;
        value
            .clone()
            .into_string()
            .map(Some)
            .map_err(|_| CliError::usage("arguments must be valid UTF-8"))
    }

    fn required_value(&mut self, option: &str) -> Result<String, CliError> {
        self.next_string()?
            .ok_or_else(|| CliError::usage(format!("{option} requires a value")))
    }
}

#[derive(Debug)]
struct CliError {
    exit_code: u8,
    code: &'static str,
    message: String,
}

impl CliError {
    fn usage(message: impl Into<String>) -> Self {
        Self {
            exit_code: EXIT_USAGE_OR_IO,
            code: "CLI_USAGE",
            message: message.into(),
        }
    }

    fn validation(message: impl Into<String>) -> Self {
        Self {
            exit_code: EXIT_VALIDATION,
            code: "CLI_VALIDATION",
            message: message.into(),
        }
    }

    fn execution(message: impl Into<String>) -> Self {
        Self::execution_with_code("CLI_EXECUTION", message)
    }

    fn execution_with_code(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            exit_code: EXIT_EXECUTION,
            code,
            message: message.into(),
        }
    }

    fn io(operation: &'static str, path: &Path, error: io::Error) -> Self {
        Self {
            exit_code: EXIT_USAGE_OR_IO,
            code: "CLI_IO",
            message: format!(
                "operation={operation}, path={}, error={error}",
                path.display()
            ),
        }
    }

    fn printed_help() -> Self {
        Self {
            exit_code: EXIT_OK,
            code: "CLI_HELP",
            message: "help displayed".to_owned(),
        }
    }
}

impl fmt::Display for CliError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

fn print_help() {
    println!(
        "promptgen {}\n\nUSAGE:\n  promptgen <COMMAND> [OPTIONS]\n\nCOMMANDS:\n  screen        compile a typed screen design and bounded UpAgent handoff\n  image         compile an image prompt or execute it with Codex image generation\n  interview     ask only for missing image contract fields and compile when complete\n  serve         run the loopback-only image Studio\n  mcp           expose the stateless image MCP compiler over stdio or Streamable HTTP\n  schema        print image/interview/compilation JSON Schema\n  catalog       list the six representative image outcomes\n  lut-presets   list resolved photographic LUT profiles\n  version       print the version\n\nRun `promptgen <COMMAND> --help` for command options.",
        env!("CARGO_PKG_VERSION")
    );
}

fn print_interview_help() {
    println!(
        "USAGE: promptgen interview [--input FILE|-] [--format json|text] [--result FILE] [--force]"
    );
}

fn print_image_help() {
    println!(
        "USAGE: promptgen image [--input FILE|-] [--mode prompt-only|luna-refine|codex-imagegen] [--format json|text] [--result FILE] [--image-output FILE] [--reference-image FILE] [--codex-binary FILE] [--codex-home DIR] [--timeout-seconds 1..3600] [--artifact-wait-seconds 1..600] [--force]"
    );
}

fn print_catalog_help() {
    println!("USAGE: promptgen catalog [--format json|text] [--result FILE] [--force]");
}

fn print_schema_help() {
    println!("USAGE: promptgen schema <image|interview|compilation> [--result FILE] [--force]");
}

fn print_serve_help() {
    println!(
        "USAGE: promptgen serve [--bind 127.0.0.1:4173] [--output-dir DIR] [--catalog-dir DIR] [--codex-binary FILE] [--codex-home DIR] [--timeout-seconds 1..3600] [--artifact-wait-seconds 1..600]"
    );
}

fn print_mcp_help() {
    println!(
        "USAGE: promptgen mcp [--transport stdio|http] [--bind 127.0.0.1:4174] [--bearer-token TOKEN] [--allowed-host HOST[,HOST...]] [--max-body-bytes 1..={MAX_BODY_BYTES}]\n\nDefault transport is stdio. HTTP is stateless-only and advertises MCP protocol 2026-07-28. Prefer PROMPTGEN_MCP_BEARER_TOKEN over the command-line token for remote binds."
    );
}

fn print_lut_help() {
    println!("USAGE: promptgen lut-presets [--result FILE] [--force]");
}

fn print_screen_help() {
    println!(
        "USAGE: promptgen screen [--input FILE|-] [--format json|text] [--result FILE] [--force]"
    );
}
fn compile_screen(options: CompileOptions) -> Result<u8, CliError> {
    let input = read_input(&options.input)?;
    let result = promptgen_core::screen::compile(&input).map_err(CliError::validation)?;
    let text = match options.format {
        OutputFormat::Json => result.to_pretty_string(),
        OutputFormat::Text => result
            .as_object()
            .and_then(|o| o.get("prompt"))
            .and_then(JsonValue::as_str)
            .ok_or_else(|| CliError::validation("missing compiled prompt"))?
            .to_owned(),
    };
    write_output(&text, options.result.as_deref(), options.force)?;
    Ok(EXIT_OK)
}
