#![forbid(unsafe_code)]

use std::borrow::Cow;
use std::io;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;

use axum::extract::State;
use axum::http::{Request, StatusCode, Uri};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use promptgen_core::catalog::catalog_json;
use promptgen_core::image::available_lut_presets;
use promptgen_core::json::{JsonValue, parse};
use promptgen_core::{
    CompilationStatus, ImagePromptRequest, InterviewRequest, InterviewStatus, compilation_schema,
    compile_image_prompt, image_schema, interview_schema, run_interview,
};
use rmcp::model::{
    CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, ErrorCode, ErrorData,
    Implementation, JsonObject, ListToolsResult, PaginatedRequestParams, ProtocolVersion,
    ResultType, ServerCapabilities, ServerInfo, Tool, ToolAnnotations,
};
use rmcp::service::RequestContext;
use rmcp::transport::streamable_http_server::session::local::LocalSessionManager;
use rmcp::transport::{StreamableHttpServerConfig, StreamableHttpService, stdio};
use rmcp::{RoleServer, ServerHandler, ServiceExt};
use serde_json::{Value, json};

pub const MCP_PROTOCOL_VERSION: &str = "2026-07-28";
pub const DEFAULT_MCP_PORT: u16 = 4174;
pub const DEFAULT_MAX_BODY_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_BODY_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct McpHttpConfig {
    pub bind: SocketAddr,
    pub bearer_token: Option<String>,
    pub allowed_hosts: Vec<String>,
    pub max_body_bytes: usize,
}

impl Default for McpHttpConfig {
    fn default() -> Self {
        Self {
            bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), DEFAULT_MCP_PORT),
            bearer_token: None,
            allowed_hosts: Vec::new(),
            max_body_bytes: DEFAULT_MAX_BODY_BYTES,
        }
    }
}

impl McpHttpConfig {
    fn validate(&self) -> io::Result<()> {
        if self.max_body_bytes == 0 || self.max_body_bytes > MAX_BODY_BYTES {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("MCP body limit must be within 1..={MAX_BODY_BYTES} bytes"),
            ));
        }
        if self.bearer_token.as_deref().is_some_and(str::is_empty) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "MCP bearer token must not be empty",
            ));
        }
        if self.allowed_hosts.iter().any(|host| host.trim().is_empty()) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "MCP allowed hosts must not contain empty values",
            ));
        }
        for host in &self.allowed_hosts {
            allowed_host_origin(host, self.bind.port())?;
        }
        if !self.bind.ip().is_loopback() {
            if self.bearer_token.is_none() {
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "non-loopback MCP binds require a bearer token",
                ));
            }
            if self.allowed_hosts.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "non-loopback MCP binds require at least one --allowed-host",
                ));
            }
        }
        Ok(())
    }
}

/// Build the stateless MCP HTTP router without binding a socket.
///
/// Every POST is handled as a self-contained request. No MCP session store is
/// installed, and the server advertises only protocol 2026-07-28.
pub fn build_router(config: McpHttpConfig) -> io::Result<Router> {
    config.validate()?;
    let allowed_origins = effective_allowed_origins(&config)?;

    let mut server_config = StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_stateless_protocol_metadata_required(true)
        .with_max_request_body_bytes(config.max_body_bytes)
        .with_allowed_origins(allowed_origins);
    if !config.allowed_hosts.is_empty() {
        server_config = server_config.with_allowed_hosts(config.allowed_hosts.clone());
    }

    let service = StreamableHttpService::new(
        || Ok(PromptGenServer),
        Arc::new(LocalSessionManager::default()),
        server_config,
    );
    Ok(Router::new()
        .route("/health", get(health))
        .nest_service("/mcp", service)
        .layer(middleware::from_fn_with_state(
            config.bearer_token,
            require_bearer,
        )))
}

fn effective_allowed_origins(config: &McpHttpConfig) -> io::Result<Vec<String>> {
    let port = config.bind.port();
    if !config.allowed_hosts.is_empty() {
        return config
            .allowed_hosts
            .iter()
            .map(|host| allowed_host_origin(host, port))
            .collect();
    }

    // An empty host list leaves rmcp's loopback Host defaults in force. Mirror
    // exactly that policy for Origin instead of creating a second authority.
    Ok(vec![
        format!("http://localhost:{port}"),
        format!("http://127.0.0.1:{port}"),
        format!("http://[::1]:{port}"),
    ])
}

fn allowed_host_origin(host: &str, default_port: u16) -> io::Result<String> {
    let host = host.trim();
    if host.is_empty()
        || host.contains("://")
        || host.contains('/')
        || host.contains('?')
        || host.contains('#')
        || host.contains('@')
        || host.chars().any(char::is_whitespace)
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid MCP allowed host: {host:?}"),
        ));
    }

    let authority = if host.parse::<IpAddr>().is_ok() && host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_owned()
    };
    let candidate = format!("http://{authority}");
    let uri = candidate.parse::<Uri>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid MCP allowed host: {host:?}"),
        )
    })?;
    if !matches!(uri.path(), "" | "/") || uri.query().is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid MCP allowed host: {host:?}"),
        ));
    }
    let authority = uri.authority().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid MCP allowed host: {host:?}"),
        )
    })?;
    let normalized_host = authority
        .host()
        .trim_start_matches('[')
        .trim_end_matches(']');
    if normalized_host.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid MCP allowed host: {host:?}"),
        ));
    }
    let port = authority.port_u16().unwrap_or(default_port);
    let normalized_host = match normalized_host.parse::<IpAddr>() {
        Ok(IpAddr::V4(address)) => address.to_string(),
        Ok(IpAddr::V6(address)) => format!("[{address}]"),
        Err(_) => normalized_host.to_ascii_lowercase(),
    };
    Ok(format!("http://{normalized_host}:{port}"))
}

/// Run the stateless Streamable HTTP MCP endpoint.
pub async fn serve_http(mut config: McpHttpConfig) -> io::Result<()> {
    config.validate()?;
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    config.bind = listener.local_addr()?;
    config.validate()?;
    let router = build_router(config)?;
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(io::Error::other)
}

/// Run the standard MCP stdio transport.
pub async fn serve_stdio() -> Result<(), String> {
    let running = PromptGenServer
        .serve(stdio())
        .await
        .map_err(|error| error.to_string())?;
    running.waiting().await.map_err(|error| error.to_string())?;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let Ok(mut signal) =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        else {
            let _ = tokio::signal::ctrl_c().await;
            return;
        };
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = signal.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

async fn health() -> impl IntoResponse {
    Json(json!({
        "service": "promptgen-mcp",
        "protocol_version": MCP_PROTOCOL_VERSION,
        "stateless": true,
        "version": env!("CARGO_PKG_VERSION"),
    }))
}

async fn require_bearer(
    State(expected): State<Option<String>>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    let Some(expected) = expected else {
        return next.run(request).await;
    };
    let authorized = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == format!("Bearer {expected}"));
    if authorized {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            [(axum::http::header::WWW_AUTHENTICATE, "Bearer")],
            "MCP bearer authentication required",
        )
            .into_response()
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PromptGenServer;

impl ServerHandler for PromptGenServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_protocol_version(ProtocolVersion::V_2026_07_28)
            .with_server_info(Implementation::new(
                "promptgen-mcp",
                env!("CARGO_PKG_VERSION"),
            ))
            .with_instructions(
                "Image-only, stateless prompt compiler. Discover the server, then use promptgen_interview when an image brief is not yet a complete typed request: call it with {kind:\"image\", brief:<brief>, answers:{}}; when structuredContent.status is needs_input, ask each structuredContent.questions entry and call again with the same brief and accumulated answers keyed by question id. When status is ready, use structuredContent.compilation.prompt as the final prompt (also mirrored in content[0].text); invalid is fail-closed. Modern Streamable HTTP non-initialize requests carry protocol 2026-07-28 metadata and headers; stdio uses newline-delimited JSON and has no HTTP headers.",
            )
    }

    fn supported_protocol_versions(&self) -> Cow<'static, [ProtocolVersion]> {
        Cow::Owned(vec![ProtocolVersion::V_2026_07_28])
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        let arguments = request.arguments.unwrap_or_default();
        match request.name.as_ref() {
            "promptgen_compile_image" => {
                let value = decode_arguments(arguments)?;
                let request = ImagePromptRequest::from_json(value)
                    .map_err(|error| tool_input_error(error.to_string()))?;
                Ok(compilation_response(compile_image_prompt(&request)))
            }
            "promptgen_interview" => {
                let value = decode_arguments(arguments)?;
                let request = InterviewRequest::from_json(value)
                    .map_err(|error| tool_input_error(error.to_string()))?;
                let outcome = run_interview(&request);
                let text = interview_to_text(&outcome);
                let is_error = outcome.status == InterviewStatus::Invalid;
                Ok(json_response(outcome.to_json(), text, is_error))
            }
            "promptgen_catalog" => {
                reject_extra_arguments(&arguments)?;
                let value =
                    catalog_json().map_err(|error| tool_internal_error(error.to_string()))?;
                let text = value.to_pretty_string();
                Ok(json_response(value, text, false))
            }
            "promptgen_lut_presets" => {
                reject_extra_arguments(&arguments)?;
                let value = available_lut_presets();
                let text = value.to_pretty_string();
                Ok(json_response(value, text, false))
            }
            name => Err(ErrorData::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("unknown promptGen MCP tool: {name}"),
                None,
            )),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        Ok(ListToolsResult::with_all_items(tool_definitions()))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        tool_definitions()
            .into_iter()
            .find(|tool| tool.name == name)
    }
}

fn tool_definitions() -> Vec<Tool> {
    vec![
        tool(
            "promptgen_compile_image",
            "Compile a validated image-generation prompt without executing an image backend.",
            image_schema(),
            Some(compilation_schema()),
        ),
        tool(
            "promptgen_interview",
            "Run the stateless guided interview; returns missing questions or a compiled prompt.",
            interview_schema(),
            Some(interview_output_schema()),
        ),
        tool(
            "promptgen_catalog",
            "Read the complete immutable promptGen image outcome catalog.",
            empty_object_schema(),
            None,
        ),
        tool(
            "promptgen_lut_presets",
            "List the resolved photographic LUT presets available to image prompt compilation.",
            empty_object_schema(),
            None,
        ),
    ]
}

fn tool(
    name: &'static str,
    description: &'static str,
    input: JsonValue,
    output: Option<JsonValue>,
) -> Tool {
    let mut tool = Tool::new(name, description, schema_arc(input));
    if let Some(output) = output {
        tool = tool.with_raw_output_schema(schema_arc(output));
    }
    tool.with_annotations(ToolAnnotations::new().read_only(true).idempotent(true))
}

fn schema_arc(value: JsonValue) -> Arc<JsonObject> {
    let value: Value = serde_json::from_str(&value.to_compact_string())
        .expect("canonical promptGen schema must be valid JSON");
    let Value::Object(value) = value else {
        panic!("canonical promptGen schema must be a JSON object");
    };
    Arc::new(value)
}

fn empty_object_schema() -> JsonValue {
    JsonValue::object([
        ("additionalProperties", JsonValue::from(false)),
        ("type", JsonValue::from("object")),
    ])
}

fn interview_output_schema() -> JsonValue {
    json_to_core_value(json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "additionalProperties": false,
        "properties": {
            "compilation": {"type": ["object", "null"]},
            "inferences": {"type": "array", "items": {"type": "string"}},
            "kind": {"enum": ["image"], "type": "string"},
            "normalized_answers": {"type": "object", "additionalProperties": {"type": "string"}},
            "questions": {"type": "array", "items": {"type": "object"}},
            "request": {"type": ["object", "null"]},
            "status": {"enum": ["needs_input", "ready", "invalid"], "type": "string"}
        },
        "required": ["compilation", "inferences", "kind", "normalized_answers", "questions", "request", "status"],
        "type": "object"
    }))
}

fn json_to_core_value(value: Value) -> JsonValue {
    parse(&value.to_string()).expect("MCP adapter schema must be valid JSON")
}

fn decode_arguments(arguments: JsonObject) -> Result<JsonValue, ErrorData> {
    let value = Value::Object(arguments);
    parse(&value.to_string()).map_err(|error| tool_input_error(error.to_string()))
}

fn reject_extra_arguments(arguments: &JsonObject) -> Result<(), ErrorData> {
    if arguments.is_empty() {
        Ok(())
    } else {
        Err(tool_input_error(format!(
            "unexpected arguments: {}",
            arguments.keys().cloned().collect::<Vec<_>>().join(", ")
        )))
    }
}

fn compilation_response(outcome: promptgen_core::CompilationOutcome) -> CallToolResponse {
    let is_error = outcome.status == CompilationStatus::Invalid;
    json_response(outcome.to_json(), outcome.to_text(), is_error)
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

fn tool_input_error(message: impl Into<String>) -> ErrorData {
    ErrorData::invalid_params(message.into(), None)
}

fn tool_internal_error(message: impl Into<String>) -> ErrorData {
    ErrorData::internal_error(message.into(), None)
}

fn json_response(value: JsonValue, text: String, is_error: bool) -> CallToolResponse {
    let value = serde_json::from_str(&value.to_compact_string())
        .expect("promptGen output must be valid JSON");
    let mut result = if is_error {
        CallToolResult::structured_error(value)
    } else {
        CallToolResult::structured(value)
    };
    result.content = vec![ContentBlock::text(text)];
    result.result_type = Some(ResultType::COMPLETE);
    result.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_stateless_protocol_is_advertised() {
        assert_eq!(
            PromptGenServer.supported_protocol_versions().as_ref(),
            &[ProtocolVersion::V_2026_07_28]
        );
        assert_eq!(
            PromptGenServer.get_info().protocol_version,
            ProtocolVersion::V_2026_07_28
        );
    }

    #[test]
    fn instructions_explain_the_stateless_interview_output_contract() {
        let instructions = PromptGenServer
            .get_info()
            .instructions
            .expect("server instructions");
        for required in [
            "promptgen_interview",
            "structuredContent.questions",
            "structuredContent.compilation.prompt",
            "invalid is fail-closed",
            "stdio uses newline-delimited JSON",
        ] {
            assert!(
                instructions.contains(required),
                "server instructions omit {required:?}: {instructions}"
            );
        }
    }

    #[test]
    fn remote_bind_requires_authentication_and_host_allowlist() {
        let config = McpHttpConfig {
            bind: "0.0.0.0:4174".parse().expect("socket address"),
            ..McpHttpConfig::default()
        };
        assert_eq!(
            config
                .validate()
                .expect_err("remote bind must be protected")
                .kind(),
            io::ErrorKind::PermissionDenied
        );
    }

    #[test]
    fn explicit_loopback_hosts_are_the_origin_policy_source() {
        let config = McpHttpConfig {
            bind: "127.0.0.1:4174".parse().expect("socket address"),
            allowed_hosts: vec!["studio.test".to_owned()],
            ..McpHttpConfig::default()
        };
        assert_eq!(
            effective_allowed_origins(&config).expect("derived origins"),
            vec!["http://studio.test:4174".to_owned()]
        );
    }

    #[test]
    fn remote_bind_derives_origins_from_existing_host_contract() {
        let config = McpHttpConfig {
            bind: "0.0.0.0:4174".parse().expect("socket address"),
            bearer_token: Some("token".to_owned()),
            allowed_hosts: vec!["EXAMPLE.test".to_owned(), "example.test:8080".to_owned()],
            ..McpHttpConfig::default()
        };
        assert_eq!(
            effective_allowed_origins(&config).expect("derived origins"),
            vec![
                "http://example.test:4174".to_owned(),
                "http://example.test:8080".to_owned(),
            ]
        );
    }

    #[test]
    fn origin_derivation_rejects_non_host_values() {
        for invalid in [
            "https://example.test",
            "example.test/path",
            "example.test?query=1",
            "example.test value",
            "user@example.test",
        ] {
            assert!(
                allowed_host_origin(invalid, 4174).is_err(),
                "host should be rejected: {invalid}"
            );
        }
        assert_eq!(
            allowed_host_origin("::1", 4174).expect("IPv6 origin"),
            "http://[::1]:4174"
        );
        assert_eq!(
            allowed_host_origin("[::1]:4175", 4174).expect("bracketed IPv6 origin"),
            "http://[::1]:4175"
        );
    }

    #[test]
    fn tool_definitions_use_canonical_schemas() {
        let tools = tool_definitions();
        assert_eq!(tools.len(), 4);
        assert!(tools.iter().all(|tool| {
            tool.annotations
                .as_ref()
                .is_some_and(|annotations| annotations.is_idempotent())
        }));
        assert!(
            tools
                .iter()
                .any(|tool| tool.name == "promptgen_compile_image")
        );
    }
}
