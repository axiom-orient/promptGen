# MCP 연결·운영

`promptgen-mcp`는 image compiler의 네 read-only 도구를 제공한다. provider를 실행하지 않는다.

| 도구 | 입력·반환 |
|---|---|
| `promptgen_compile_image` | typed image 요청 → compilation/진단 |
| `promptgen_interview` | brief+answers → needs_input/ready/invalid |
| `promptgen_catalog` | 빈 object → 전체 outcome catalog |
| `promptgen_lut_presets` | 빈 object → LUT 목록 |

## 시작

```sh
cargo build --release -p promptgen-cli --locked --offline
./target/release/promptgen mcp
./target/release/promptgen mcp --transport http
```

기본은 newline-delimited JSON stdio다. HTTP 기본은 `127.0.0.1:4174`. non-loopback은 bearer와 최소 하나의 `--allowed-host HOST`가 필요하다. bearer는 `PROMPTGEN_MCP_BEARER_TOKEN` 사용을 권장한다. body 기본 4 MiB, 최대 16 MiB다.

## protocol·보안 계약

protocol은 `2026-07-28`이다. HTTP client는 protocol metadata/header와 Content-Type/Accept를 맞춰야 한다. domain-invalid, JSON-RPC error, HTTP error는 구분한다.

HTTP Origin은 Host와 별도 검증한다. public `McpHttpConfig`에 중복 정책 값을 추가하지 않고, 현재 실제 bind port와 기존 `allowed_hosts`에서 허용 HTTP Origin을 결정적으로 파생해 **rmcp 3.1.0의 `with_allowed_origins` validator 한 곳**에 전달한다.

- loopback: `http://localhost:PORT`, `http://127.0.0.1:PORT`, `http://[::1]:PORT`
- non-loopback: 각 `allowed_host`의 명시 port 또는 실제 bind port를 사용한 `http://HOST:PORT`
- Origin 없음: MCP client 호환을 위해 rmcp 규칙대로 허용
- allowlist 밖 Origin / `null`: 403
- scheme·authority를 파싱할 수 없는 Origin: rmcp 3.1.0 동작대로 400
- URI parser가 tuple로 읽더라도 allowlist와 다른 path가 붙은 Origin: 일치하지 않는 값으로 403

server 자체는 plain HTTP다. TLS reverse proxy의 외부 `https://` Origin을 허용하는 별도 public 설정은 현재 contract에 없다. 그 배포 형태가 필요해질 때 기존 public config를 깨뜨리지 않는 확장 계약을 별도로 결정한다.

공식 근거는 [MCP Streamable HTTP 2026-07-28](https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http)과 [rmcp 3.1.0 source](https://docs.rs/rmcp/3.1.0/src/rmcp/transport/streamable_http_server/tower.rs.html), 확인일 2026-09-13이다.

## 검증

```sh
cargo test -p promptgen-mcp --test streamable_http --locked --offline
cargo test -p promptgen-cli --test cli mcp_stdio_external_agent_interview_retries_questions_and_returns_prompt --locked --offline
```

`verify.sh`는 release build·schema check·MCP HTTP integration을 포함한다. 실제 현재 실행 결과는 [구현 현황](IMPLEMENTATION_STATUS.md)과 [VERIFICATION.json](../VERIFICATION.json)을 기준으로 한다. 테스트에 사용한 provider executable은 transport/adapter 경계용 test fixture이며 외부 provider 성공을 증명하지 않는다.
