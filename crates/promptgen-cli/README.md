# promptgen-cli

`promptgen` binary의 composition root다. domain 규칙을 core에 위임하고 파일/stdio·옵션·exit·서비스 수명을 연결한다.

| 명령 | 역할 |
|---|---|
| `image` | 기본 결정적 prompt-only, `luna-refine` 검토, Luna 선검토 후 Codex 구독 생성 또는 단일 base PNG 편집 |
| `interview` | brief와 누적 answers를 받아 다음 질문 또는 완성 요청 반환 |
| `screen` | 화면 raw JSON을 prompt 또는 원문/hash 포함 handoff로 컴파일 |
| `schema`, `catalog`, `lut-presets` | 지원 schema·catalog·LUT 조회 |
| `serve`, `mcp` | Studio 또는 MCP 시작 |
| `help`, `version` | 명령·버전 조회 |

옵션·경로·입출력 계약의 원본은 [main.rs](src/main.rs), 전체 시작 방법은 [루트 README](../../README.md)다.
정상 0, usage/I/O 2, validation 3, provider 실행 오류 4를 구별한다.
exit만으로 원격 미실행·PNG 미게시를 추정하지 않는다. 현재 alias/부분 게시 위험은 [ANALYSIS](ANALYSIS.md)에 있다.
Luna review는 설치된 Codex CLI와 `gpt-5.6-luna`를 사용한다. `--codex-binary`, `--codex-home`, timeout 설정은 검토와 Codex 실행에 함께 적용된다. `--codex-sha256`을 지정하면 절대경로의 실행 파일이 regular non-symlink인지와 SHA-256을 매 child 실행 전에 다시 확인한다. 실행 receipt에는 검증한 digest가 들어간다. `codex-imagegen`의 후보 횟수는 `--max-fidelity-attempts 1..=4`로 제한할 수 있으며 기본값은 4다. API key는 요청·사용하지 않으며 Codex 실행에 `forced_login_method="chatgpt"`를 강제한다.
