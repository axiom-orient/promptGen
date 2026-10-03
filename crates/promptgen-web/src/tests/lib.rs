use super::*;
use std::io::Cursor;
use std::net::{IpAddr, Shutdown};

fn state() -> ServerState {
    let output_dir = std::env::temp_dir().join("promptgen-web-tests");
    ServerState {
        config: WebConfig {
            bind: "127.0.0.1:4173".parse().unwrap(),
            output_dir: output_dir.clone(),
            ..WebConfig::default()
        },
        catalog_dir: resolve_catalog_dir(None),
        local_addr: "127.0.0.1:4173".parse().unwrap(),
        generation_busy: AtomicBool::new(false),
        active_connections: AtomicUsize::new(0),
    }
}

fn parse_request(raw: &str) -> Result<HttpRequest, RequestError> {
    HttpRequest::read_from(&mut Cursor::new(raw.as_bytes()), DEFAULT_MAX_BODY_BYTES)
}

struct OneByteReader {
    bytes: Vec<u8>,
    offset: usize,
}

impl Read for OneByteReader {
    fn read(&mut self, output: &mut [u8]) -> std::io::Result<usize> {
        if self.offset == self.bytes.len() || output.is_empty() {
            return Ok(0);
        }
        output[0] = self.bytes[self.offset];
        self.offset += 1;
        Ok(1)
    }
}

#[test]
fn request_parser_finds_header_boundary_without_rescanning_prior_bytes() {
    let raw = b"GET /health HTTP/1.1\r\nHost: localhost:4173\r\n\r\n";
    let mut reader = OneByteReader {
        bytes: raw.to_vec(),
        offset: 0,
    };
    let request = HttpRequest::read_from(&mut reader, DEFAULT_MAX_BODY_BYTES).unwrap();
    assert_eq!(request.method, "GET");
    assert_eq!(request.path, "/health");
}

#[test]
fn serve_listener_revalidates_the_actual_bound_socket() {
    let listener = TcpListener::bind("0.0.0.0:0").unwrap();
    let config = WebConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        ..WebConfig::default()
    };
    let error = serve_listener(listener, config, Arc::new(AtomicBool::new(true))).unwrap_err();
    assert_eq!(error.code, "WEB_BIND_NOT_LOOPBACK");
}

#[test]
fn refuses_non_loopback_bind() {
    let config = WebConfig {
        bind: "0.0.0.0:4173".parse().unwrap(),
        ..WebConfig::default()
    };
    assert_eq!(
        validate_config(&config).unwrap_err().code,
        "WEB_BIND_NOT_LOOPBACK"
    );
}

#[test]
fn post_requires_client_header_and_json_content_type() {
    let body = r#"{"kind":"image","brief":"poster"}"#;
    let raw = format!(
        "POST /api/v3/interview HTTP/1.1\r\nHost: localhost:4173\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let response = route(parse_request(&raw).unwrap(), &state());
    assert_eq!(response.status, 403);
}

#[test]
fn interview_returns_missing_questions() {
    let body = r#"{"kind":"image","brief":"한국어 커피 포스터","answers":{}}"#;
    let raw = format!(
        "POST /api/v3/interview HTTP/1.1\r\nHost: localhost:4173\r\nX-PromptGen-Client: web-v3\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        body
    );
    let response = route(parse_request(&raw).unwrap(), &state());
    assert_eq!(response.status, 200);
    let text = String::from_utf8(response.body).unwrap();
    assert!(text.contains("needs_input"));
    assert!(text.contains("image.subject"));
}

/// The published binary is relocatable, so the catalog root has to be runtime state.
/// A compile-time source path silently 404s every card image once the binary moves.
#[test]
fn catalog_asset_root_is_runtime_state_not_a_build_path() {
    let raw = "GET /api/v3/catalog/assets/C1 HTTP/1.1\r\nHost: localhost:4173\r\n\r\n";

    let resolved = state();
    assert!(
        resolved.catalog_dir.is_some(),
        "the checkout must resolve its own catalog directory"
    );
    let served = route(parse_request(raw).unwrap(), &resolved);
    assert_eq!(served.status, 200);
    assert_eq!(&served.body[..8], b"\x89PNG\r\n\x1a\n");

    let explicit = WebConfig {
        catalog_dir: Some(PathBuf::from("/nonexistent-catalog-root")),
        ..WebConfig::default()
    };
    assert_eq!(
        resolve_catalog_dir(explicit.catalog_dir.as_deref()),
        None,
        "an unusable explicit root is authoritative and must not silently fall back"
    );

    let unavailable = ServerState {
        catalog_dir: None,
        ..state()
    };
    let missing = route(parse_request(raw).unwrap(), &unavailable);
    assert_eq!(missing.status, 503);
    assert!(
        String::from_utf8(missing.body)
            .unwrap()
            .contains("CATALOG_ASSET_ROOT_UNAVAILABLE")
    );
}

#[test]
fn catalog_root_and_asset_bytes_are_bound_to_the_compiled_catalog() {
    let root = test_root("catalog-integrity");
    fs::write(root.join("image_catalog.json"), CATALOG_SOURCE_JSON).unwrap();
    assert_eq!(resolve_catalog_dir(Some(&root)), Some(root.clone()));

    let entry = find_entry("C1").unwrap().unwrap();
    let asset = root.join(&entry.asset_path);
    fs::create_dir_all(asset.parent().unwrap()).unwrap();
    let (expected_bytes, _) = catalog_asset_contract(&entry.asset_path).unwrap();
    fs::write(&asset, vec![0_u8; expected_bytes as usize]).unwrap();

    let mut broken = state();
    broken.catalog_dir = Some(root.clone());
    let raw = "GET /api/v3/catalog/assets/C1 HTTP/1.1\r\nHost: localhost:4173\r\n\r\n";
    let response = route(parse_request(raw).unwrap(), &broken);
    assert_eq!(response.status, 500);
    assert!(
        String::from_utf8(response.body)
            .unwrap()
            .contains("CATALOG_ASSET_INTEGRITY")
    );

    fs::write(root.join("image_catalog.json"), b"{}\n").unwrap();
    assert_eq!(resolve_catalog_dir(Some(&root)), None);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn output_name_is_fail_closed() {
    for value in ["../x.png", ".x.png", "x.jpg", "a/b.png", "x..png"] {
        assert!(validate_output_name(value).is_err(), "{value}");
    }
    assert!(validate_output_name("promptgen-123.png").is_ok());
}

#[test]
fn duplicate_content_length_is_rejected() {
    let raw = "POST /api/v3/interview HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n";
    assert_eq!(
        parse_request(raw).unwrap_err().code,
        "HTTP_DUPLICATE_HEADER"
    );
}

#[test]
fn generation_guard_is_exclusive_and_releases() {
    let busy = AtomicBool::new(false);
    let guard = GenerationGuard::acquire(&busy).expect("first guard");
    assert!(GenerationGuard::acquire(&busy).is_none());
    drop(guard);
    assert!(GenerationGuard::acquire(&busy).is_some());
}

#[test]
fn connection_guard_enforces_bound_and_releases() {
    let state = Arc::new(state());
    let guards = (0..MAX_ACTIVE_CONNECTIONS)
        .map(|_| ConnectionGuard::acquire(&state).expect("within connection limit"))
        .collect::<Vec<_>>();
    assert!(ConnectionGuard::acquire(&state).is_none());
    drop(guards);
    assert_eq!(state.active_connections.load(Ordering::Acquire), 0);
    assert!(ConnectionGuard::acquire(&state).is_some());
}

#[test]
fn allowed_host_is_loopback_only() {
    let addr: SocketAddr = "127.0.0.1:4173".parse().unwrap();
    assert!(allowed_host("localhost:4173", addr));
    assert!(allowed_host("127.0.0.1:4173", addr));
    assert!(!allowed_host("evil.example", addr));
}

#[test]
fn artifact_route_rejects_traversal() {
    let response = serve_artifact("/artifacts/../secret.png", &"0".repeat(64), &state());
    assert_eq!(response.status, 404);
}

#[test]
fn artifact_route_requires_the_exact_receipt_digest() {
    let missing = route(
        parse_request("GET /artifacts/x.png HTTP/1.1\r\nHost: localhost:4173\r\n\r\n").unwrap(),
        &state(),
    );
    assert_eq!(missing.status, 400);

    assert_eq!(
        artifact_sha256_query("/artifacts/x.png?sha256=abc")
            .unwrap_err()
            .status,
        400
    );
    assert_eq!(
        artifact_sha256_query(
            "/artifacts/x.png?sha256=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa&sha256=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
        )
        .unwrap_err()
        .status,
        400
    );
    assert_eq!(
        artifact_sha256_query(
            "/artifacts/x.png?sha256=AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
        )
        .unwrap()
        .as_str(),
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
    );
    assert_eq!(
        artifact_sha256_query(
            "/artifacts/x.png?sha256=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa&v=1"
        )
        .unwrap_err()
        .status,
        400
    );
}

#[test]
fn request_parser_enforces_body_limit() {
    let raw = "POST /api/v3/interview HTTP/1.1\r\nHost: localhost\r\nContent-Length: 100\r\n\r\n";
    let error = HttpRequest::read_from(&mut Cursor::new(raw.as_bytes()), 10).unwrap_err();
    assert_eq!(error.status, 413);
}

fn test_root(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "promptgen-web-{label}-{}-{nanos}",
        std::process::id()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

fn spawn_test_server(
    mut config: WebConfig,
) -> (
    SocketAddr,
    ShutdownSignal,
    thread::JoinHandle<Result<(), WebError>>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    config.bind = address;
    let shutdown = ShutdownSignal::new(address);
    let flag = shutdown.flag();
    let handle = thread::spawn(move || serve_listener(listener, config, flag));
    (address, shutdown, handle)
}

fn live_request(address: SocketAddr, method: &str, path: &str, body: Option<&str>) -> Vec<u8> {
    let mut stream = TcpStream::connect(address).unwrap();
    let body = body.unwrap_or_default();
    let mut request =
        format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n");
    if method == "POST" {
        request.push_str("X-PromptGen-Client: web-v3\r\n");
        request.push_str("Content-Type: application/json\r\n");
        request.push_str(&format!("Content-Length: {}\r\n", body.len()));
    }
    request.push_str("\r\n");
    request.push_str(body);
    stream.write_all(request.as_bytes()).unwrap();
    stream.shutdown(Shutdown::Write).unwrap();
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    response
}

fn response_body(response: &[u8]) -> &[u8] {
    let boundary = find_subsequence(response, b"\r\n\r\n").expect("response boundary");
    &response[boundary + 4..]
}

#[test]
fn embedded_ui_exposes_guided_controls_and_inspector() {
    assert!(INDEX_HTML.contains("id=\"inspector-panel\""));
    assert!(INDEX_HTML.contains("id=\"knowledge-list\""));
    assert!(INDEX_HTML.contains("id=\"recent-list\""));
    assert!(INDEX_HTML.contains("id=\"undo-sample\""));
    assert!(INDEX_HTML.contains("visual-shortcuts"));
    assert!(APP_JS.contains("question.control === \"multi_choice\""));
    assert!(APP_JS.contains("selectCatalogEntry"));
    assert!(APP_JS.contains("sampleBackup"));
    assert!(APP_JS.contains("catalog_entries"));
    assert!(APP_JS.contains("generation_plan"));
    assert!(APP_JS.contains("invalidateInterviewForBriefChange"));
    assert!(APP_JS.contains("catalogQuestion"));
    assert!(APP_JS.contains("prompt_directives"));
    assert!(APP_JS.contains("const HISTORY_KEY = \"promptgen.recent.v"));
    assert!(APP_JS.contains("image.profile"));
    assert!(APP_JS.contains("value: \"app_icon\""));
    assert!(APP_JS.contains("opaque 1024² PNG master concept"));
    assert!(APP_JS.contains("value: \"mixed\""));
    assert!(APP_JS.contains("selectedProfileDefaults"));
    assert!(APP_JS.contains("profileValue !== \"app_icon\""));
    assert!(APP_JS.contains("openCatalogPreview"));
    assert!(APP_JS.contains("IntersectionObserver"));
    assert!(APP_CSS.contains(".choice-grid"));
    assert!(APP_CSS.contains(".inspector-panel.open"));
    assert!(APP_CSS.contains("grid-template-columns: minmax(300px, 380px) minmax(0, 1fr)"));
    assert!(APP_CSS.contains("--card-ratio"));
    assert!(APP_CSS.contains("@media (prefers-color-scheme: dark)"));
}

#[test]
fn live_server_serves_ui_health_and_guided_interview() {
    let root = test_root("live");
    let config = WebConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        output_dir: root.join("output"),
        codex_home: root.join("codex-home"),
        ..WebConfig::default()
    };
    let (address, shutdown, handle) = spawn_test_server(config);

    let index = live_request(address, "GET", "/", None);
    assert!(index.starts_with(b"HTTP/1.1 200 OK"));
    assert!(
        std::str::from_utf8(response_body(&index))
            .unwrap()
            .contains("이미지 프롬프트 작업실")
    );

    let health = live_request(address, "GET", "/api/v3/health", None);
    assert!(health.starts_with(b"HTTP/1.1 200 OK"));
    assert!(
        std::str::from_utf8(response_body(&health))
            .unwrap()
            .contains("promptgen-web")
    );

    let body = r#"{"kind":"image","brief":"30대 성인 모델 1명의 절제된 리조트웨어 세로 화보, 골든아워 해안 테라스.","answers":{}}"#;
    let interview = live_request(address, "POST", "/api/v3/interview", Some(body));
    assert!(interview.starts_with(b"HTTP/1.1 200 OK"));
    let interview_body = std::str::from_utf8(response_body(&interview)).unwrap();
    assert!(interview_body.contains("needs_input"));
    assert!(interview_body.contains("image.wardrobe"));

    shutdown.signal();
    handle.join().unwrap().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn live_generate_with_fake_codex_publishes_verified_artifact() {
    use std::os::unix::fs::PermissionsExt;

    let root = test_root("generate");
    let fixture = root.join("fixture.png");
    fs::write(
        &fixture,
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/valid-1024x1536.png"
        )),
    )
    .unwrap();
    let script = root.join("fake-codex.sh");
    fs::write(
            &script,
            format!(
                "#!/bin/sh\nset -eu\nif [ \"${{1:-}}\" = \"--version\" ]; then echo 'codex-cli web-test'; exit 0; fi\nresult=''\nmodel=''\nprevious=''\nfor argument in \"$@\"; do\n  if [ \"$previous\" = '--output-last-message' ]; then result=\"$argument\"; fi\n  if [ \"$previous\" = '--model' ]; then model=\"$argument\"; fi\n  previous=\"$argument\"\ndone\nif [ \"$model\" = 'gpt-5.6-luna' ]; then\n  mkdir -p \"$CODEX_HOME\"\n  printf '%s\\n' '{{\"approved\":true,\"summary\":\"The test prompt is coherent.\",\"additions\":[]}}' > \"$result\"\n  : > \"$CODEX_HOME/luna-reviewed\"\n  printf '%s\\n' '{{\"type\":\"thread.started\",\"thread_id\":\"thread-luna\"}}' '{{\"type\":\"turn.completed\"}}'\n  exit 0\nfi\nif [ -n \"$result\" ]; then\n  printf '%s\\n' '{{\"pass\":true,\"summary\":\"all hard constraints pass\",\"subject_counts\":[{{\"id\":\"person\",\"expected\":1,\"observed\":1,\"evidence\":\"one model\",\"instances\":[{{\"x_percent\":50,\"y_percent\":50,\"evidence\":\"model\"}}]}}],\"text_checks\":[],\"violations\":[],\"repair_instruction\":\"\"}}' > \"$result\"\n  printf '%s\\n' '{{\"type\":\"thread.started\",\"thread_id\":\"thread-validator\"}}' '{{\"type\":\"turn.completed\"}}'\n  exit 0\nfi\n[ -f \"$CODEX_HOME/luna-reviewed\" ]\nmkdir -p \"$CODEX_HOME/generated_images/thread-web\"\ncp '{}' \"$CODEX_HOME/generated_images/thread-web/call-web.png\"\nprintf '%s\n' '{{\"type\":\"thread.started\",\"thread_id\":\"thread-web\"}}' '{{\"type\":\"turn.completed\"}}'\n",
                fixture.display()
            ),
        )
        .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();

    let config = WebConfig {
        bind: "127.0.0.1:0".parse().unwrap(),
        codex_binary: script,
        codex_home: root.join("codex-home"),
        output_dir: root.join("output"),
        request_timeout: Duration::from_secs(5),
        artifact_wait_timeout: Duration::from_secs(5),
        ..WebConfig::default()
    };
    let (address, shutdown, handle) = spawn_test_server(config);
    let body = r#"{
          "interview": {
            "kind": "image",
            "brief": "30대 성인 모델 1명의 절제된 리조트웨어 세로 화보, 골든아워 해안 테라스.",
            "answers": {
              "image.wardrobe": "세이지색 불투명 리넨 테일러드 재킷, 높은 라운드넥 크림 이너, 발목 길이 와이드 팬츠, 여유로운 핏",
              "image.adult_editorial_confirm": "yes"
            }
          },
          "mode": "codex-imagegen",
          "output_name": "web-e2e.png"
        }"#;
    let generated = live_request(address, "POST", "/api/v3/generate", Some(body));
    assert!(
        generated.starts_with(b"HTTP/1.1 200 OK"),
        "{}",
        String::from_utf8_lossy(&generated)
    );
    let generated_body = std::str::from_utf8(response_body(&generated)).unwrap();
    assert!(generated_body.contains("\"ok\":true"));
    assert!(generated_body.contains("thread-web"));

    let output_path = root.join("output/web-e2e.png");
    assert!(output_path.is_file());
    let published_sha256 = sha256_bytes(&fs::read(&output_path).unwrap());
    let bound_path = format!("/artifacts/web-e2e.png?sha256={published_sha256}");
    assert!(generated_body.contains(&bound_path));
    let artifact = live_request(address, "GET", &bound_path, None);
    assert!(artifact.starts_with(b"HTTP/1.1 200 OK"));
    assert!(response_body(&artifact).starts_with(b"\x89PNG\r\n\x1a\n"));

    fs::write(&output_path, b"mutated-after-publication").unwrap();
    let stale = live_request(address, "GET", &bound_path, None);
    assert!(stale.starts_with(b"HTTP/1.1 409 Conflict"));
    assert!(
        std::str::from_utf8(response_body(&stale))
            .unwrap()
            .contains("ARTIFACT_DIGEST_MISMATCH")
    );

    shutdown.signal();
    handle.join().unwrap().unwrap();
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn local_address_is_an_ip() {
    assert!(matches!(state().local_addr.ip(), IpAddr::V4(_)));
}
