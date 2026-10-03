#![forbid(unsafe_code)]

#[cfg(test)]
mod asset_contract;

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{self, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use promptgen_codex::{
    CodexRunConfig, LunaReviewConfig, execute_image_generation, review_image_prompt,
};
use promptgen_core::catalog::{catalog_json, find_entry};
use promptgen_core::image::available_lut_presets;
use promptgen_core::image_artifact::sha256::{Sha256, hex as hex_sha256};
use promptgen_core::interview::InterviewRequestValue;
use promptgen_core::json::{
    DecodeError, JsonValue, into_object, parse_with_limits, reject_unknown, take_optional,
    take_required, take_string,
};
use promptgen_core::{InterviewRequest, InterviewStatus, run_interview};

const INDEX_HTML: &str = include_str!("assets/index.html");
const APP_CSS: &str = include_str!("assets/app.css");
const APP_JS: &str = include_str!("assets/app.js");
const CATALOG_SOURCE_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../catalog/image_catalog.json"
));
const CATALOG_ASSET_MANIFEST_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../catalog/CHATGPT_IMAGE_CATALOG_MANIFEST.json"
));
const MAX_HEADER_BYTES: usize = 32 * 1024;
const DEFAULT_MAX_BODY_BYTES: usize = 12 * 1024 * 1024;
const MAX_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_CATALOG_ASSET_BYTES: u64 = 12 * 1024 * 1024;
const MAX_CATALOG_INDEX_BYTES: u64 = 2 * 1024 * 1024;
const READ_TIMEOUT: Duration = Duration::from_secs(15);
const MAX_ACTIVE_CONNECTIONS: usize = 32;

#[derive(Clone, Debug)]
pub struct WebConfig {
    pub bind: SocketAddr,
    pub codex_binary: PathBuf,
    pub codex_home: PathBuf,
    pub output_dir: PathBuf,
    /// Directory holding `image_catalog.json` and `assets/`. Card PNGs are read from
    /// here at runtime; `None` resolves it once at startup with [`resolve_catalog_dir`].
    pub catalog_dir: Option<PathBuf>,
    pub max_body_bytes: usize,
    pub request_timeout: Duration,
    pub artifact_wait_timeout: Duration,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            bind: "127.0.0.1:4173".parse().expect("literal socket address"),
            codex_binary: PathBuf::from("codex"),
            codex_home: default_codex_home(),
            output_dir: PathBuf::from("var/output"),
            catalog_dir: None,
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
            request_timeout: Duration::from_secs(240),
            artifact_wait_timeout: Duration::from_secs(30),
        }
    }
}

/// Resolves the directory that holds the runtime catalog assets.
///
/// The published binary is relocatable, so the compile-time source layout cannot be
/// used here. A candidate is accepted only when its runtime `image_catalog.json` is
/// byte-identical to the catalog compiled into this binary. This prevents the UI from
/// showing cards whose IDs/asset paths belong to a different catalog generation.
pub fn resolve_catalog_dir(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(path) = explicit {
        return catalog_root_matches_binary(path).then(|| path.to_path_buf());
    }
    if let Some(value) = std::env::var_os("PROMPTGEN_CATALOG_DIR") {
        let path = PathBuf::from(value);
        return catalog_root_matches_binary(&path).then_some(path);
    }

    let mut candidates: Vec<PathBuf> = Vec::new();
    // An installed binary sits next to its catalog; a source checkout runs it from
    // `target/<profile>/` or `target/debug/deps/`. Walking up covers both without
    // encoding either layout.
    if let Ok(executable) = std::env::current_exe()
        && let Some(directory) = executable.parent()
    {
        candidates.extend(directory.ancestors().map(|base| base.join("catalog")));
    }
    if let Ok(current) = std::env::current_dir() {
        candidates.extend(current.ancestors().map(|base| base.join("catalog")));
    }
    candidates
        .into_iter()
        .find(|candidate| catalog_root_matches_binary(candidate))
}

fn catalog_root_matches_binary(path: &Path) -> bool {
    read_file_bounded(&path.join("image_catalog.json"), MAX_CATALOG_INDEX_BYTES)
        .is_ok_and(|bytes| bytes == CATALOG_SOURCE_JSON.as_bytes())
}

#[derive(Debug)]
pub struct WebError {
    pub code: &'static str,
    pub message: String,
}

impl WebError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl fmt::Display for WebError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for WebError {}

impl From<io::Error> for WebError {
    fn from(error: io::Error) -> Self {
        Self::new("WEB_IO", error.to_string())
    }
}

struct ServerState {
    config: WebConfig,
    catalog_dir: Option<PathBuf>,
    local_addr: SocketAddr,
    generation_busy: AtomicBool,
    active_connections: AtomicUsize,
}

/// Stop signal for a running [`serve_listener`].
///
/// The accept loop blocks instead of polling, so the flag alone cannot end it: an
/// idle server is parked inside `accept`. Signalling therefore sets the flag and
/// opens one throwaway loopback connection, which is the wake-up event. The
/// alternative — a non-blocking listener with a sleep — burns wake-ups forever and
/// adds that sleep to the latency of every request.
#[derive(Clone, Debug)]
pub struct ShutdownSignal {
    flag: Arc<AtomicBool>,
    address: SocketAddr,
}

impl ShutdownSignal {
    pub fn new(address: SocketAddr) -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            address,
        }
    }

    pub fn flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.flag)
    }

    pub fn signal(&self) {
        self.flag.store(true, Ordering::Release);
        if let Ok(stream) = TcpStream::connect_timeout(&self.address, READ_TIMEOUT) {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }
}

pub fn serve(config: WebConfig) -> Result<(), WebError> {
    validate_config(&config)?;
    let listener = TcpListener::bind(config.bind).map_err(|error| {
        WebError::new(
            "WEB_BIND",
            format!("failed to bind {}: {error}", config.bind),
        )
    })?;
    let local_addr = listener.local_addr()?;
    eprintln!("promptGen UI: http://{local_addr}");
    serve_listener(listener, config, Arc::new(AtomicBool::new(false)))
}

pub fn serve_listener(
    listener: TcpListener,
    mut config: WebConfig,
    shutdown: Arc<AtomicBool>,
) -> Result<(), WebError> {
    validate_config(&config)?;
    let local_addr = listener.local_addr()?;
    config.bind = local_addr;
    validate_config(&config)?;
    fs::create_dir_all(&config.output_dir).map_err(|error| {
        WebError::new(
            "WEB_OUTPUT_DIR",
            format!("failed to create {}: {error}", config.output_dir.display()),
        )
    })?;
    let catalog_dir = resolve_catalog_dir(config.catalog_dir.as_deref());
    if catalog_dir.is_none() {
        eprintln!(
            "promptGen: no catalog directory found; card images are unavailable. \
             Set PROMPTGEN_CATALOG_DIR or run from a directory containing catalog/image_catalog.json."
        );
    }
    let state = Arc::new(ServerState {
        config,
        catalog_dir,
        local_addr,
        generation_busy: AtomicBool::new(false),
        active_connections: AtomicUsize::new(0),
    });
    let mut workers = Vec::new();
    let mut terminal_error = None;
    loop {
        reap_finished_workers(&mut workers);
        // Checked before dispatching rather than around a poll: the wake-up
        // connection from `ShutdownSignal` is what returns control here.
        if shutdown.load(Ordering::Acquire) {
            break;
        }
        match listener.accept() {
            Ok((mut stream, _peer)) => {
                if shutdown.load(Ordering::Acquire) {
                    break;
                }
                let Some(connection_guard) = ConnectionGuard::acquire(&state) else {
                    let rejection = (|| -> io::Result<()> {
                        stream.set_write_timeout(Some(READ_TIMEOUT))?;
                        Response::json_error(
                            503,
                            "WEB_CONNECTION_LIMIT",
                            "too many concurrent local connections",
                        )
                        .write_to(&mut stream)
                    })();
                    if let Err(error) = rejection {
                        eprintln!("promptGen connection-limit response error: {error}");
                    }
                    continue;
                };
                let worker_state = Arc::clone(&state);
                match thread::Builder::new()
                    .name("promptgen-http".to_owned())
                    .spawn(move || {
                        let _connection_guard = connection_guard;
                        if let Err(error) = handle_connection(stream, &worker_state) {
                            eprintln!("promptGen web request error: {error}");
                        }
                    }) {
                    Ok(worker) => workers.push(worker),
                    Err(error) => eprintln!("promptGen failed to spawn request worker: {error}"),
                }
            }
            // A connection can die between the kernel queueing it and `accept`
            // returning; that is the peer's failure, not the server's.
            Err(error)
                if matches!(
                    error.kind(),
                    io::ErrorKind::ConnectionAborted | io::ErrorKind::Interrupted
                ) => {}
            Err(error) => {
                terminal_error = Some(WebError::from(error));
                break;
            }
        }
    }
    drain_workers(workers);
    terminal_error.map_or(Ok(()), Err)
}

fn reap_finished_workers(workers: &mut Vec<thread::JoinHandle<()>>) {
    let mut index = 0;
    while index < workers.len() {
        if workers[index].is_finished() {
            let worker = workers.swap_remove(index);
            if worker.join().is_err() {
                eprintln!("promptGen web request worker panicked");
            }
        } else {
            index += 1;
        }
    }
}

fn drain_workers(workers: Vec<thread::JoinHandle<()>>) {
    for worker in workers {
        if worker.join().is_err() {
            eprintln!("promptGen web request worker panicked during shutdown");
        }
    }
}

/// The catalog is immutable for the life of the process, so its response body is
/// built once. Rebuilding and re-serializing the catalog on every
/// Studio load is pure waste: the bytes are identical every time.
fn catalog_response_body() -> Result<&'static String, &'static String> {
    static BODY: OnceLock<Result<String, String>> = OnceLock::new();
    BODY.get_or_init(|| {
        let value = catalog_json().map_err(|error| error.to_string())?;
        Ok(JsonValue::object([
            ("ok", JsonValue::from(true)),
            ("catalog", value),
            (
                "visual_controls",
                promptgen_core::image::controls::controls_json(),
            ),
        ])
        .to_compact_string())
    })
    .as_ref()
}

fn validate_config(config: &WebConfig) -> Result<(), WebError> {
    if !config.bind.ip().is_loopback() {
        return Err(WebError::new(
            "WEB_BIND_NOT_LOOPBACK",
            format!("refusing non-loopback bind address {}", config.bind),
        ));
    }
    if config.max_body_bytes == 0 || config.max_body_bytes > 16 * 1024 * 1024 {
        return Err(WebError::new(
            "WEB_BODY_LIMIT",
            "max_body_bytes must be within 1..=16777216",
        ));
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream, state: &ServerState) -> Result<(), WebError> {
    stream.set_read_timeout(Some(READ_TIMEOUT))?;
    stream.set_write_timeout(Some(READ_TIMEOUT))?;
    let request = match HttpRequest::read_from(&mut stream, state.config.max_body_bytes) {
        Ok(request) => request,
        Err(error) => {
            let response = Response::json_error(error.status, error.code, error.message);
            response.write_to(&mut stream)?;
            return Ok(());
        }
    };
    let response = route(request, state);
    response.write_to(&mut stream)?;
    Ok(())
}

#[derive(Debug)]
struct HttpRequest {
    method: String,
    path: String,
    version: String,
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug)]
struct RequestError {
    status: u16,
    code: &'static str,
    message: String,
}

impl RequestError {
    fn new(status: u16, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }
}

impl HttpRequest {
    fn read_from(reader: &mut impl Read, max_body_bytes: usize) -> Result<Self, RequestError> {
        const HEADER_TERMINATOR: &[u8] = b"\r\n\r\n";
        let mut buffer = Vec::with_capacity(4096);
        let mut chunk = [0_u8; 2048];
        let mut search_start = 0usize;
        let header_end = loop {
            if buffer.len() > MAX_HEADER_BYTES {
                return Err(RequestError::new(
                    431,
                    "HTTP_HEADERS_TOO_LARGE",
                    "request headers exceed 32768 bytes",
                ));
            }
            if let Some(index) = find_subsequence(&buffer[search_start..], HEADER_TERMINATOR) {
                break search_start + index + HEADER_TERMINATOR.len();
            }
            let previous_len = buffer.len();
            let read = reader.read(&mut chunk).map_err(|error| {
                RequestError::new(400, "HTTP_READ", format!("failed to read request: {error}"))
            })?;
            if read == 0 {
                return Err(RequestError::new(
                    400,
                    "HTTP_INCOMPLETE_HEADERS",
                    "connection ended before complete headers",
                ));
            }
            buffer.extend_from_slice(&chunk[..read]);
            search_start = previous_len.saturating_sub(HEADER_TERMINATOR.len() - 1);
        };

        let head = std::str::from_utf8(&buffer[..header_end - 4]).map_err(|_| {
            RequestError::new(400, "HTTP_HEADER_ENCODING", "headers must be valid UTF-8")
        })?;
        let mut lines = head.split("\r\n");
        let request_line = lines.next().unwrap_or_default();
        let parts = request_line.split_whitespace().collect::<Vec<_>>();
        if parts.len() != 3 {
            return Err(RequestError::new(
                400,
                "HTTP_REQUEST_LINE",
                "request line must contain method, path, and HTTP version",
            ));
        }
        let method = parts[0].to_owned();
        let path = parts[1].to_owned();
        let version = parts[2].to_owned();
        if version != "HTTP/1.1" && version != "HTTP/1.0" {
            return Err(RequestError::new(
                505,
                "HTTP_VERSION",
                "only HTTP/1.0 and HTTP/1.1 are supported",
            ));
        }
        if !path.starts_with('/') || path.contains('\0') {
            return Err(RequestError::new(400, "HTTP_PATH", "invalid request path"));
        }

        let mut headers = BTreeMap::new();
        for line in lines {
            let Some((name, value)) = line.split_once(':') else {
                return Err(RequestError::new(
                    400,
                    "HTTP_HEADER",
                    format!("malformed header line {line:?}"),
                ));
            };
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().to_owned();
            if name.is_empty() || headers.insert(name.clone(), value).is_some() {
                return Err(RequestError::new(
                    400,
                    "HTTP_DUPLICATE_HEADER",
                    format!("duplicate or empty header {name:?}"),
                ));
            }
        }
        if headers.contains_key("transfer-encoding") {
            return Err(RequestError::new(
                400,
                "HTTP_TRANSFER_ENCODING",
                "chunked or transformed request bodies are not supported",
            ));
        }
        let content_length = match headers.get("content-length") {
            Some(value) => value.parse::<usize>().map_err(|_| {
                RequestError::new(400, "HTTP_CONTENT_LENGTH", "invalid Content-Length")
            })?,
            None if method == "POST" => {
                return Err(RequestError::new(
                    411,
                    "HTTP_LENGTH_REQUIRED",
                    "POST requires Content-Length",
                ));
            }
            None => 0,
        };
        if content_length > max_body_bytes {
            return Err(RequestError::new(
                413,
                "HTTP_BODY_TOO_LARGE",
                format!("request body exceeds {max_body_bytes} bytes"),
            ));
        }
        if method != "POST" && content_length != 0 {
            return Err(RequestError::new(
                400,
                "HTTP_UNEXPECTED_BODY",
                "only POST requests may contain a body",
            ));
        }
        let already = buffer.len().saturating_sub(header_end);
        if already > content_length {
            buffer.truncate(header_end + content_length);
        }
        while buffer.len() - header_end < content_length {
            let remaining = content_length - (buffer.len() - header_end);
            let read_size = remaining.min(chunk.len());
            let read = reader.read(&mut chunk[..read_size]).map_err(|error| {
                RequestError::new(
                    400,
                    "HTTP_BODY_READ",
                    format!("failed to read body: {error}"),
                )
            })?;
            if read == 0 {
                return Err(RequestError::new(
                    400,
                    "HTTP_INCOMPLETE_BODY",
                    "connection ended before Content-Length bytes were read",
                ));
            }
            buffer.extend_from_slice(&chunk[..read]);
        }
        Ok(Self {
            method,
            path,
            version,
            headers,
            body: buffer[header_end..header_end + content_length].to_vec(),
        })
    }
}

fn route(request: HttpRequest, state: &ServerState) -> Response {
    if let Err(response) = validate_request_authority(&request, state) {
        return response;
    }
    let path = request.path.split('?').next().unwrap_or(&request.path);
    match (request.method.as_str(), path) {
        ("GET", "/") | ("GET", "/index.html") => {
            Response::text(200, "text/html; charset=utf-8", INDEX_HTML)
        }
        ("GET", "/app.css") => Response::text(200, "text/css; charset=utf-8", APP_CSS),
        ("GET", "/app.js") => Response::text(200, "text/javascript; charset=utf-8", APP_JS),
        ("GET", "/api/v3/health") => Response::json(
            200,
            JsonValue::object([
                ("ok", JsonValue::from(true)),
                ("service", JsonValue::from("promptgen-web")),
                ("version", JsonValue::from(env!("CARGO_PKG_VERSION"))),
            ]),
        ),
        ("GET", "/api/v3/catalog") => match catalog_response_body() {
            Ok(body) => Response::bytes(
                200,
                "application/json; charset=utf-8",
                body.as_bytes().to_vec(),
            ),
            Err(error) => Response::json_error(500, "CATALOG_LOAD", error.clone()),
        },
        ("GET", asset_path) if asset_path.starts_with("/api/v3/catalog/assets/") => {
            serve_catalog_asset(&request, asset_path, state)
        }
        ("GET", "/api/v3/lut-presets") => Response::json(
            200,
            JsonValue::object([
                ("ok", JsonValue::from(true)),
                ("presets", available_lut_presets()),
            ]),
        ),
        ("POST", "/api/v3/interview") => handle_interview(&request, state),
        ("POST", "/api/v3/generate") => handle_generate(&request, state),
        ("GET", artifact_path) if artifact_path.starts_with("/artifacts/") => {
            match artifact_sha256_query(&request.path) {
                Ok(expected_sha256) => serve_artifact(artifact_path, &expected_sha256, state),
                Err(response) => response,
            }
        }
        _ => Response::json_error(404, "HTTP_NOT_FOUND", "route not found"),
    }
}

fn read_file_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, io::Error> {
    let file = fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(maximum + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > maximum {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file grew beyond the configured read limit",
        ));
    }
    Ok(bytes)
}

fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_sha256(&hasher.finalize())
}

fn catalog_asset_contract(asset_path: &str) -> Result<(u64, String), String> {
    let manifest = parse_with_limits(CATALOG_ASSET_MANIFEST_JSON, 256 * 1024, 32)
        .map_err(|error| format!("embedded catalog asset manifest is invalid: {error}"))?;
    let images = manifest
        .as_object()
        .and_then(|root| root.get("images"))
        .and_then(JsonValue::as_array)
        .ok_or_else(|| "embedded catalog asset manifest is missing images".to_owned())?;
    for image in images {
        let Some(fields) = image.as_object() else {
            return Err("embedded catalog asset manifest contains a non-object image".to_owned());
        };
        if fields.get("asset_path").and_then(JsonValue::as_str) != Some(asset_path) {
            continue;
        }
        let bytes = fields
            .get("bytes")
            .and_then(JsonValue::as_u64)
            .ok_or_else(|| format!("catalog asset {asset_path} has no byte contract"))?;
        let sha256 = fields
            .get("sha256")
            .and_then(JsonValue::as_str)
            .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or_else(|| format!("catalog asset {asset_path} has no SHA-256 contract"))?;
        return Ok((bytes, sha256.to_ascii_lowercase()));
    }
    Err(format!(
        "catalog asset {asset_path} has no embedded integrity contract"
    ))
}

fn serve_catalog_asset(request: &HttpRequest, path: &str, state: &ServerState) -> Response {
    let id = path.trim_start_matches("/api/v3/catalog/assets/");
    if id.is_empty()
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Response::json_error(404, "CATALOG_ASSET_NOT_FOUND", "catalog asset not found");
    }
    let entry = match find_entry(id) {
        Ok(Some(entry)) => entry,
        Ok(None) | Err(_) => {
            return Response::json_error(404, "CATALOG_ASSET_NOT_FOUND", "catalog asset not found");
        }
    };
    let (expected_bytes, expected_sha256) = match catalog_asset_contract(&entry.asset_path) {
        Ok(contract) => contract,
        Err(error) => return Response::json_error(500, "CATALOG_ASSET_CONTRACT", error),
    };
    if expected_bytes == 0 || expected_bytes > MAX_CATALOG_ASSET_BYTES {
        return Response::json_error(
            500,
            "CATALOG_ASSET_CONTRACT",
            "catalog asset byte contract exceeds the server limit",
        );
    }
    let Some(root) = state.catalog_dir.as_ref() else {
        return Response::json_error(
            503,
            "CATALOG_ASSET_ROOT_UNAVAILABLE",
            "no matching catalog directory is configured; set PROMPTGEN_CATALOG_DIR to the catalog packaged with this binary",
        );
    };
    let asset = root.join(&entry.asset_path);
    let metadata = match fs::metadata(&asset) {
        Ok(metadata) if metadata.is_file() && metadata.len() == expected_bytes => metadata,
        Ok(metadata) if metadata.is_file() && metadata.len() > MAX_CATALOG_ASSET_BYTES => {
            return Response::json_error(
                413,
                "CATALOG_ASSET_TOO_LARGE",
                "catalog asset exceeds its read limit",
            );
        }
        Ok(_) | Err(_) => {
            return Response::json_error(404, "CATALOG_ASSET_NOT_FOUND", "catalog asset not found");
        }
    };
    debug_assert_eq!(metadata.len(), expected_bytes);
    let bytes = match read_file_bounded(&asset, MAX_CATALOG_ASSET_BYTES) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::InvalidData => {
            return Response::json_error(
                413,
                "CATALOG_ASSET_TOO_LARGE",
                "catalog asset exceeds its read limit",
            );
        }
        Err(_) => {
            return Response::json_error(404, "CATALOG_ASSET_NOT_FOUND", "catalog asset not found");
        }
    };
    if bytes.len() as u64 != expected_bytes || sha256_bytes(&bytes) != expected_sha256 {
        return Response::json_error(
            500,
            "CATALOG_ASSET_INTEGRITY",
            "catalog asset bytes do not match the manifest compiled into this binary",
        );
    }
    let validator = format!("\"sha256-{expected_sha256}\"");
    if request.headers.get("if-none-match").is_some_and(|value| {
        value
            .split(',')
            .any(|candidate| candidate.trim() == validator)
    }) {
        return Response::not_modified(validator);
    }
    Response::cached_bytes("image/png", bytes, validator)
}

fn validate_request_authority(request: &HttpRequest, state: &ServerState) -> Result<(), Response> {
    let host = request
        .headers
        .get("host")
        .map(String::as_str)
        .unwrap_or_default();
    if request.version == "HTTP/1.1" && host.is_empty() {
        return Err(Response::json_error(
            400,
            "HTTP_HOST_REQUIRED",
            "Host header is required",
        ));
    }
    if !host.is_empty() && !allowed_host(host, state.local_addr) {
        return Err(Response::json_error(
            403,
            "HTTP_HOST_REJECTED",
            "Host header is not a loopback promptGen origin",
        ));
    }
    if request.method == "POST" {
        if request
            .headers
            .get("x-promptgen-client")
            .map(String::as_str)
            != Some("web-v3")
        {
            return Err(Response::json_error(
                403,
                "HTTP_CLIENT_HEADER",
                "POST requires X-PromptGen-Client: web-v3",
            ));
        }
        if !request
            .headers
            .get("content-type")
            .is_some_and(|value| value.to_ascii_lowercase().starts_with("application/json"))
        {
            return Err(Response::json_error(
                415,
                "HTTP_CONTENT_TYPE",
                "POST requires application/json",
            ));
        }
        if let Some(origin) = request.headers.get("origin")
            && !allowed_origin(origin, state.local_addr)
        {
            return Err(Response::json_error(
                403,
                "HTTP_ORIGIN_REJECTED",
                "Origin is not the local promptGen UI",
            ));
        }
    }
    Ok(())
}

fn handle_interview(request: &HttpRequest, state: &ServerState) -> Response {
    match decode_interview(&request.body, state.config.max_body_bytes) {
        Ok(interview) => {
            let outcome = run_interview(&interview);
            Response::json(
                200,
                JsonValue::object([
                    (
                        "ok",
                        JsonValue::from(outcome.status != InterviewStatus::Invalid),
                    ),
                    ("interview", outcome.to_json()),
                ]),
            )
        }
        Err(error) => Response::json_error(400, "INTERVIEW_DECODE", error.to_string()),
    }
}

fn handle_generate(request: &HttpRequest, state: &ServerState) -> Response {
    let generation = match GenerateRequest::decode(&request.body, state.config.max_body_bytes) {
        Ok(value) => value,
        Err(error) => return Response::json_error(400, "GENERATE_DECODE", error.to_string()),
    };
    let outcome = run_interview(&generation.interview);
    if outcome.status == InterviewStatus::NeedsInput {
        return Response::json(
            200,
            JsonValue::object([
                ("ok", JsonValue::from(false)),
                ("interview", outcome.to_json()),
                ("reason", JsonValue::from("needs_input")),
            ]),
        );
    }
    if outcome.status == InterviewStatus::Invalid {
        return Response::json(
            422,
            JsonValue::object([
                ("ok", JsonValue::from(false)),
                ("interview", outcome.to_json()),
                ("reason", JsonValue::from("invalid_compilation")),
            ]),
        );
    }
    if generation.mode == GenerationMode::PromptOnly {
        return Response::json(
            200,
            JsonValue::object([
                ("ok", JsonValue::from(true)),
                ("interview", outcome.to_json()),
                ("mode", JsonValue::from(generation.mode.as_str())),
            ]),
        );
    }
    // The interview already decoded and compiled this request; both are taken from
    // it together, so "ready with nothing to run" is not a state this route can see.
    let Some(ready) = outcome.ready() else {
        return Response::json_error(
            500,
            "GENERATE_NOT_READY",
            "interview is not in a ready state",
        );
    };
    let InterviewRequestValue::Image(image_request) = ready.request;
    let compilation = ready.compilation;
    if image_request.task_mode != promptgen_core::image::ImageTaskMode::Generate
        || !image_request.references.is_empty()
    {
        return Response::json_error(
            422,
            "WEB_REFERENCE_INPUT_REQUIRED",
            "Studio generation accepts no reference images. Use codex-imagegen --reference-image with the actual base file for this compiled edit.",
        );
    }
    let output_name = generation.output_name.unwrap_or_else(default_output_name);
    if let Err(message) = validate_output_name(&output_name) {
        return Response::json_error(400, "GENERATE_OUTPUT_NAME", message);
    }
    let _guard = match GenerationGuard::acquire(&state.generation_busy) {
        Some(guard) => guard,
        None => {
            return Response::json_error(
                409,
                "GENERATE_BUSY",
                "another image generation is already running",
            );
        }
    };
    let mut luna_config = LunaReviewConfig::new(
        state.config.codex_binary.clone(),
        state.config.codex_home.clone(),
    );
    luna_config.timeout = state.config.request_timeout;
    let refinement = match review_image_prompt(compilation, image_request, &luna_config) {
        Ok(refinement) => refinement,
        Err(error) => {
            let status = if error.code == "CODEX_LUNA_REVIEW_REJECTED" {
                422
            } else {
                502
            };
            return Response::json(
                status,
                JsonValue::object([
                    (
                        "error",
                        JsonValue::object([
                            ("code", JsonValue::from(error.code)),
                            ("message", JsonValue::from(error.message)),
                        ]),
                    ),
                    ("ok", JsonValue::from(false)),
                ]),
            );
        }
    };
    let output_path = state.config.output_dir.join(&output_name);
    let config = CodexRunConfig {
        codex_binary: state.config.codex_binary.clone(),
        codex_home: state.config.codex_home.clone(),
        output_path,
        timeout: state.config.request_timeout,
        artifact_wait_timeout: state.config.artifact_wait_timeout,
        max_fidelity_attempts: 4,
        overwrite: false,
        reference_image: None,
    };
    match execute_image_generation(compilation, image_request, &refinement, &config) {
        Ok(receipt) => Response::json(
            200,
            JsonValue::object([
                (
                    "artifact_url",
                    JsonValue::from(format!(
                        "/artifacts/{output_name}?sha256={}",
                        receipt.sha256
                    )),
                ),
                ("interview", outcome.to_json()),
                ("mode", JsonValue::from(generation.mode.as_str())),
                ("ok", JsonValue::from(true)),
                ("receipt", receipt.to_json()),
            ]),
        ),
        Err(error) => Response::json(
            502,
            JsonValue::object([
                (
                    "error",
                    JsonValue::object([
                        ("code", JsonValue::from(error.code)),
                        ("message", JsonValue::from(error.message)),
                    ]),
                ),
                ("interview", outcome.to_json()),
                ("ok", JsonValue::from(false)),
                ("prompt_refinement", refinement.to_json()),
            ]),
        ),
    }
}

fn artifact_sha256_query(request_path: &str) -> Result<String, Response> {
    let Some((_, query)) = request_path.split_once('?') else {
        return Err(Response::json_error(
            400,
            "ARTIFACT_DIGEST_QUERY",
            "artifact URL must include exactly one sha256 query parameter",
        ));
    };
    let mut expected = None;
    for pair in query.split('&').filter(|pair| !pair.is_empty()) {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        match key {
            "sha256" => {
                if expected.is_some() {
                    return Err(Response::json_error(
                        400,
                        "ARTIFACT_DIGEST_QUERY",
                        "sha256 query parameter must appear exactly once",
                    ));
                }
                if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                    return Err(Response::json_error(
                        400,
                        "ARTIFACT_DIGEST_QUERY",
                        "sha256 query parameter must be exactly 64 hexadecimal characters",
                    ));
                }
                expected = Some(value.to_ascii_lowercase());
            }
            _ => {
                return Err(Response::json_error(
                    400,
                    "ARTIFACT_DIGEST_QUERY",
                    "only one sha256 query parameter is allowed",
                ));
            }
        }
    }
    expected.ok_or_else(|| {
        Response::json_error(
            400,
            "ARTIFACT_DIGEST_QUERY",
            "artifact URL must include exactly one sha256 query parameter",
        )
    })
}

fn serve_artifact(path: &str, expected_sha256: &str, state: &ServerState) -> Response {
    let name = path.trim_start_matches("/artifacts/");
    if validate_output_name(name).is_err() {
        return Response::json_error(404, "ARTIFACT_NOT_FOUND", "artifact not found");
    }
    let path = state.config.output_dir.join(name);
    let metadata = match fs::symlink_metadata(&path) {
        Ok(value) if value.file_type().is_file() && !value.file_type().is_symlink() => value,
        _ => return Response::json_error(404, "ARTIFACT_NOT_FOUND", "artifact not found"),
    };
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Response::json_error(413, "ARTIFACT_TOO_LARGE", "artifact exceeds 64 MiB");
    }
    match read_file_bounded(&path, MAX_ARTIFACT_BYTES) {
        Ok(bytes) => {
            let observed_sha256 = sha256_bytes(&bytes);
            if observed_sha256 != expected_sha256 {
                return Response::json_error(
                    409,
                    "ARTIFACT_DIGEST_MISMATCH",
                    format!(
                        "artifact bytes no longer match the requested receipt digest: expected={expected_sha256}, observed={observed_sha256}"
                    ),
                );
            }
            Response::bytes(200, "image/png", bytes)
        }
        Err(error) if error.kind() == io::ErrorKind::InvalidData => {
            Response::json_error(413, "ARTIFACT_TOO_LARGE", "artifact exceeds 64 MiB")
        }
        Err(error) => Response::json_error(500, "ARTIFACT_READ", error.to_string()),
    }
}

fn decode_interview(body: &[u8], max_body_bytes: usize) -> Result<InterviewRequest, WebError> {
    let text = std::str::from_utf8(body)
        .map_err(|_| WebError::new("JSON_ENCODING", "JSON body must be UTF-8"))?;
    let value = parse_with_limits(text, max_body_bytes, 96)
        .map_err(|error| WebError::new("JSON_PARSE", error.to_string()))?;
    InterviewRequest::from_json(value)
        .map_err(|error| WebError::new("INTERVIEW_SCHEMA", error.to_string()))
}

struct ConnectionGuard {
    state: Arc<ServerState>,
}

impl ConnectionGuard {
    fn acquire(state: &Arc<ServerState>) -> Option<Self> {
        state
            .active_connections
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                (current < MAX_ACTIVE_CONNECTIONS).then_some(current + 1)
            })
            .ok()?;
        Some(Self {
            state: Arc::clone(state),
        })
    }
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        let previous = self.state.active_connections.fetch_sub(1, Ordering::AcqRel);
        debug_assert!(previous > 0, "connection counter underflow");
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GenerationMode {
    PromptOnly,
    CodexImagegen,
}

impl GenerationMode {
    fn parse(value: &str) -> Result<Self, DecodeError> {
        match value {
            "prompt-only" => Ok(Self::PromptOnly),
            "codex-imagegen" => Ok(Self::CodexImagegen),
            _ => Err(DecodeError::new(
                "$.mode",
                "expected prompt-only or codex-imagegen",
            )),
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::PromptOnly => "prompt-only",
            Self::CodexImagegen => "codex-imagegen",
        }
    }
}

struct GenerateRequest {
    interview: InterviewRequest,
    mode: GenerationMode,
    output_name: Option<String>,
}

impl GenerateRequest {
    fn decode(body: &[u8], max_body_bytes: usize) -> Result<Self, WebError> {
        let text = std::str::from_utf8(body)
            .map_err(|_| WebError::new("JSON_ENCODING", "JSON body must be UTF-8"))?;
        let value = parse_with_limits(text, max_body_bytes, 96)
            .map_err(|error| WebError::new("JSON_PARSE", error.to_string()))?;
        let mut fields = into_object(value, "$generate")
            .map_err(|error| WebError::new("GENERATE_SCHEMA", error.to_string()))?;
        let interview_value = take_required(&mut fields, "interview", "$generate")
            .map_err(|error| WebError::new("GENERATE_SCHEMA", error.to_string()))?;
        let interview = InterviewRequest::from_json(interview_value)
            .map_err(|error| WebError::new("GENERATE_SCHEMA", error.to_string()))?;
        let mode = take_string(&mut fields, "mode", "$generate")
            .and_then(|value| GenerationMode::parse(&value))
            .map_err(|error| WebError::new("GENERATE_SCHEMA", error.to_string()))?;
        let output_name = match take_optional(&mut fields, "output_name") {
            None | Some(JsonValue::Null) => None,
            Some(JsonValue::String(value)) => Some(value),
            Some(_) => {
                return Err(WebError::new(
                    "GENERATE_SCHEMA",
                    "$.output_name must be a string or null",
                ));
            }
        };
        reject_unknown(fields, "$generate")
            .map_err(|error| WebError::new("GENERATE_SCHEMA", error.to_string()))?;
        Ok(Self {
            interview,
            mode,
            output_name,
        })
    }
}

struct GenerationGuard<'a> {
    busy: &'a AtomicBool,
}

impl<'a> GenerationGuard<'a> {
    fn acquire(busy: &'a AtomicBool) -> Option<Self> {
        busy.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| Self { busy })
    }
}

impl Drop for GenerationGuard<'_> {
    fn drop(&mut self) {
        self.busy.store(false, Ordering::Release);
    }
}

fn validate_output_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 128 {
        return Err("output_name length must be within 1..=128".to_owned());
    }
    if !name.ends_with(".png") {
        return Err("output_name must end with .png".to_owned());
    }
    if name.starts_with('.') || name.contains("..") {
        return Err("output_name may not be hidden or contain parent traversal".to_owned());
    }
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(
            "output_name may contain only ASCII letters, digits, '-', '_', and '.'".to_owned(),
        );
    }
    Ok(())
}

fn default_output_name() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("promptgen-{millis}.png")
}

fn default_codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".codex")))
        .unwrap_or_else(|| PathBuf::from(".codex"))
}

fn allowed_host(value: &str, local_addr: SocketAddr) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    let host = if lower.starts_with('[') {
        lower.split(']').next().map(|value| format!("{value}]"))
    } else {
        lower.split(':').next().map(ToOwned::to_owned)
    };
    matches!(host.as_deref(), Some("localhost" | "127.0.0.1" | "[::1]"))
        || host.as_deref() == Some(local_addr.ip().to_string().as_str())
}

fn allowed_origin(value: &str, local_addr: SocketAddr) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    [
        format!("http://localhost:{}", local_addr.port()),
        format!("http://127.0.0.1:{}", local_addr.port()),
        format!("http://[::1]:{}", local_addr.port()),
        format!("http://{}", local_addr),
    ]
    .contains(&lower)
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[derive(Debug)]
struct Response {
    status: u16,
    content_type: &'static str,
    body: Vec<u8>,
    cache_control: &'static str,
    etag: Option<String>,
}

impl Response {
    fn text(status: u16, content_type: &'static str, body: &str) -> Self {
        Self {
            status,
            content_type,
            body: body.as_bytes().to_vec(),
            cache_control: "no-store",
            etag: None,
        }
    }

    fn bytes(status: u16, content_type: &'static str, body: Vec<u8>) -> Self {
        Self {
            status,
            content_type,
            body,
            cache_control: "private, max-age=60",
            etag: None,
        }
    }

    /// Catalog art is immutable for a given build, and a single grid pulls tens of megabytes.
    /// A validator plus a long private max-age keeps tab switches and reloads off the wire.
    fn cached_bytes(content_type: &'static str, body: Vec<u8>, etag: String) -> Self {
        Self {
            status: 200,
            content_type,
            body,
            cache_control: "private, max-age=86400",
            etag: Some(etag),
        }
    }

    fn not_modified(etag: String) -> Self {
        Self {
            status: 304,
            content_type: "application/octet-stream",
            body: Vec::new(),
            cache_control: "private, max-age=86400",
            etag: Some(etag),
        }
    }

    fn json(status: u16, value: JsonValue) -> Self {
        Self {
            status,
            content_type: "application/json; charset=utf-8",
            body: value.to_compact_string().into_bytes(),
            cache_control: "no-store",
            etag: None,
        }
    }

    fn json_error(status: u16, code: &'static str, message: impl Into<String>) -> Self {
        Self::json(
            status,
            JsonValue::object([
                (
                    "error",
                    JsonValue::object([
                        ("code", JsonValue::from(code)),
                        ("message", JsonValue::from(message.into())),
                    ]),
                ),
                ("ok", JsonValue::from(false)),
            ]),
        )
    }

    fn write_to(&self, writer: &mut impl Write) -> io::Result<()> {
        let etag = match &self.etag {
            Some(value) => format!("ETag: {value}\r\n"),
            None => String::new(),
        };
        write!(
            writer,
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: {}\r\n{}Connection: close\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nX-Frame-Options: DENY\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data: blob:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'\r\n\r\n",
            self.status,
            reason_phrase(self.status),
            self.content_type,
            self.body.len(),
            self.cache_control,
            etag,
        )?;
        writer.write_all(&self.body)?;
        writer.flush()
    }
}

const fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        304 => "Not Modified",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Content",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        505 => "HTTP Version Not Supported",
        _ => "Error",
    }
}

#[cfg(test)]
#[path = "tests/lib.rs"]
mod tests;
