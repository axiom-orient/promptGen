use std::net::SocketAddr;

use promptgen_mcp::{MCP_PROTOCOL_VERSION, McpHttpConfig, build_router};
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[tokio::test]
async fn stateless_discover_lists_only_the_supported_protocol() {
    let (address, server) = start_server(None).await;
    let metadata = request_metadata();
    let body = json_rpc(1, "server/discover", json!({"_meta": metadata}));
    let response = send(address, "server/discover", None, &body).await;

    assert_eq!(response.status, 200, "{}", response.body);
    let value: Value = serde_json::from_str(&response.body).expect("JSON response");
    assert_eq!(
        value["result"]["supportedVersions"],
        json!([MCP_PROTOCOL_VERSION])
    );
    assert_eq!(
        value["result"]["_meta"]["io.modelcontextprotocol/serverInfo"]["name"],
        "promptgen-mcp"
    );
    server.abort();
}

#[tokio::test]
async fn stateless_tools_list_and_call_return_canonical_compilation() {
    let (address, server) = start_server(None).await;
    let metadata = request_metadata();
    let list_body = json_rpc(2, "tools/list", json!({"_meta": metadata}));
    let list = send(address, "tools/list", None, &list_body).await;
    assert_eq!(list.status, 200, "{}", list.body);
    assert!(list.body.contains("promptgen_compile_image"));

    let input: Value = serde_json::from_str(include_str!("../../../examples/image-photo-lut.json"))
        .expect("image example JSON");
    let call_body = json_rpc(
        3,
        "tools/call",
        json!({
            "_meta": request_metadata(),
            "name": "promptgen_compile_image",
            "arguments": input,
        }),
    );
    let call = send(
        address,
        "tools/call",
        Some("promptgen_compile_image"),
        &call_body,
    )
    .await;
    assert_eq!(call.status, 200, "{}", call.body);
    let value: Value = serde_json::from_str(&call.body).expect("JSON response");
    assert_eq!(value["result"]["isError"], false);
    assert_eq!(value["result"]["structuredContent"]["status"], "valid");
    assert!(
        value["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("PURPOSE") || text.contains("목적"))
    );
    server.abort();
}

#[tokio::test]
async fn external_agent_interview_retries_questions_and_returns_prompt() {
    let (address, server) = start_server(None).await;
    let brief = "30대 성인 모델 1명의 절제된 리조트웨어 세로 화보, 골든아워 해안 테라스.";
    let first_body = json_rpc(
        10,
        "tools/call",
        json!({
            "_meta": request_metadata(),
            "name": "promptgen_interview",
            "arguments": {"kind": "image", "brief": brief, "answers": {}},
        }),
    );
    let first = send(
        address,
        "tools/call",
        Some("promptgen_interview"),
        &first_body,
    )
    .await;
    assert_eq!(first.status, 200, "{}", first.body);
    let first_value: Value = serde_json::from_str(&first.body).expect("first JSON response");
    assert_eq!(first_value["result"]["isError"], false);
    assert_eq!(
        first_value["result"]["structuredContent"]["status"],
        "needs_input"
    );
    assert_eq!(
        first_value["result"]["structuredContent"]["questions"][0]["id"],
        "image.wardrobe"
    );

    let second_body = json_rpc(
        11,
        "tools/call",
        json!({
            "_meta": request_metadata(),
            "name": "promptgen_interview",
            "arguments": {
                "kind": "image",
                "brief": brief,
                "answers": {
                    "image.wardrobe": "세이지색 불투명 리넨 테일러드 재킷, 높은 라운드넥 크림 이너, 발목 길이 와이드 팬츠, 여유로운 핏"
                }
            },
        }),
    );
    let second = send(
        address,
        "tools/call",
        Some("promptgen_interview"),
        &second_body,
    )
    .await;
    assert_eq!(second.status, 200, "{}", second.body);
    let second_value: Value = serde_json::from_str(&second.body).expect("second JSON response");
    assert_eq!(second_value["result"]["isError"], false);
    assert_eq!(
        second_value["result"]["structuredContent"]["status"],
        "ready"
    );
    let prompt = second_value["result"]["structuredContent"]["compilation"]["prompt"]
        .as_str()
        .expect("ready interview includes compilation.prompt");
    assert!(!prompt.is_empty());
    assert!(prompt.contains("세이지색 불투명 리넨"));
    assert!(
        second_value["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("세이지색 불투명 리넨"))
    );
    server.abort();
}

#[tokio::test]
async fn invalid_interview_request_is_rejected_without_compilation() {
    let (address, server) = start_server(None).await;
    let body = json_rpc(
        12,
        "tools/call",
        json!({
            "_meta": request_metadata(),
            "name": "promptgen_interview",
            "arguments": {"kind": "audio", "brief": "not an image", "answers": {}},
        }),
    );
    let response = send(address, "tools/call", Some("promptgen_interview"), &body).await;
    assert_eq!(response.status, 400, "{}", response.body);
    let value: Value = serde_json::from_str(&response.body).expect("JSON response");
    assert_eq!(value["error"]["code"], -32602);
    assert!(
        value["error"]["message"]
            .as_str()
            .is_some_and(|message| { message.contains("expected image") })
    );
    assert!(value["result"].is_null());
    server.abort();
}

#[tokio::test]
async fn stateless_mode_rejects_missing_request_metadata_and_checks_bearer_auth() {
    let (address, server) = start_server(Some("test-token")).await;
    let body = json_rpc(4, "tools/list", json!({}));
    let unauthorized = send(address, "tools/list", None, &body).await;
    assert_eq!(unauthorized.status, 401, "{}", unauthorized.body);

    let missing_metadata =
        send_with_token(address, "tools/list", None, &body, "test-token", false).await;
    assert_eq!(missing_metadata.status, 400, "{}", missing_metadata.body);

    let valid = send_with_token(
        address,
        "tools/list",
        None,
        &json_rpc(5, "tools/list", json!({"_meta": request_metadata()})),
        "test-token",
        false,
    )
    .await;
    assert_eq!(valid.status, 200, "{}", valid.body);
    server.abort();
}

#[tokio::test]
async fn streamable_http_rejects_unallowed_origins_and_accepts_same_origin() {
    let (address, server) = start_server(None).await;
    let body = json_rpc(20, "tools/list", json!({"_meta": request_metadata()}));

    let rejected = send_with_origin(
        address,
        "tools/list",
        None,
        &body,
        Some("https://evil.example"),
    )
    .await;
    assert_eq!(rejected.status, 403, "{}", rejected.body);

    let null_origin = send_with_origin(address, "tools/list", None, &body, Some("null")).await;
    assert_eq!(null_origin.status, 403, "{}", null_origin.body);

    let malformed = send_with_origin(
        address,
        "tools/list",
        None,
        &body,
        Some("https://example.test/path"),
    )
    .await;
    assert_eq!(malformed.status, 403, "{}", malformed.body);

    let unparsable =
        send_with_origin(address, "tools/list", None, &body, Some("not-an-origin")).await;
    assert_eq!(unparsable.status, 400, "{}", unparsable.body);

    let origin = format!("http://127.0.0.1:{}", address.port());
    let accepted = send_with_origin(address, "tools/list", None, &body, Some(&origin)).await;
    assert_eq!(accepted.status, 200, "{}", accepted.body);
    server.abort();
}

struct HttpResponse {
    status: u16,
    body: String,
}

async fn start_server(token: Option<&str>) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind test server");
    let address = listener.local_addr().expect("test server address");
    let config = McpHttpConfig {
        bind: address,
        bearer_token: token.map(ToOwned::to_owned),
        ..McpHttpConfig::default()
    };
    let router = build_router(config).expect("build MCP router");
    let server = tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("serve test router");
    });
    (address, server)
}

fn request_metadata() -> Value {
    json!({
        "io.modelcontextprotocol/protocolVersion": MCP_PROTOCOL_VERSION,
        "io.modelcontextprotocol/clientInfo": {"name": "promptgen-test", "version": "1"},
        "io.modelcontextprotocol/clientCapabilities": {},
    })
}

fn json_rpc(id: u64, method: &str, params: Value) -> String {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params}).to_string()
}

async fn send(address: SocketAddr, method: &str, name: Option<&str>, body: &str) -> HttpResponse {
    send_with_token(address, method, name, body, "", false).await
}

async fn send_with_origin(
    address: SocketAddr,
    method: &str,
    name: Option<&str>,
    body: &str,
    origin: Option<&str>,
) -> HttpResponse {
    send_request(address, method, name, body, "", false, origin).await
}

async fn send_with_token(
    address: SocketAddr,
    method: &str,
    name: Option<&str>,
    body: &str,
    token: &str,
    omit_protocol_headers: bool,
) -> HttpResponse {
    send_request(
        address,
        method,
        name,
        body,
        token,
        omit_protocol_headers,
        None,
    )
    .await
}

async fn send_request(
    address: SocketAddr,
    method: &str,
    name: Option<&str>,
    body: &str,
    token: &str,
    omit_protocol_headers: bool,
    origin: Option<&str>,
) -> HttpResponse {
    let mut stream = TcpStream::connect(address)
        .await
        .expect("connect test server");
    let mut headers = format!(
        "Host: 127.0.0.1:{}\r\nContent-Type: application/json\r\nAccept: application/json, text/event-stream\r\nMcp-Method: {method}\r\n",
        address.port()
    );
    if let Some(name) = name {
        headers.push_str(&format!("Mcp-Name: {name}\r\n"));
    }
    if !omit_protocol_headers {
        headers.push_str(&format!("MCP-Protocol-Version: {MCP_PROTOCOL_VERSION}\r\n"));
    }
    if !token.is_empty() {
        headers.push_str(&format!("Authorization: Bearer {token}\r\n"));
    }
    if let Some(origin) = origin {
        headers.push_str(&format!("Origin: {origin}\r\n"));
    }
    let request = format!(
        "POST /mcp HTTP/1.1\r\n{headers}Connection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream
        .write_all(request.as_bytes())
        .await
        .expect("write HTTP request");
    let mut bytes = Vec::new();
    stream
        .read_to_end(&mut bytes)
        .await
        .expect("read HTTP response");
    let response = String::from_utf8(bytes).expect("UTF-8 HTTP response");
    let status = response
        .split_whitespace()
        .nth(1)
        .expect("HTTP status")
        .parse()
        .expect("numeric HTTP status");
    let body = response
        .split_once("\r\n\r\n")
        .map_or_else(String::new, |(_, body)| body.to_owned());
    HttpResponse { status, body }
}
