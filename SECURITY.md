# 보안·신뢰 경계

## 신뢰하지 않는 입력

사용자 JSON·guided answers, HTTP header/body/path, Codex JSONL과 생성 파일,
외부 reference PNG, 목적지의 기존 파일/링크 상태를 신뢰하지 않는다.
컴파일 prompt는 실행 권한이나 인증 수단이 아니다. 실제 방어 구현과 남은 gap은 [Current](docs/IMPLEMENTATION_STATUS.md)에 연결한다.

## 적용 경계와 알려진 한계

| 경계 | 방어 | 한계·필수 주의 |
|---|---|---|
| core 입력 | JSON byte/depth·duplicate/unknown field·enum/range/교차 검증, invalid prompt 거부 | public typed API의 전제는 JSON 진입과 다를 수 있음. PNG 파일 편의 API는 실제 FS read를 수행 |
| Web | 기본 loopback, 실제 listener 주소 재검증, Host/optional Origin, JSON·`X-PromptGen-Client: web-v3`, header/body/time limit, 단일 generation 제한, async request identity | custom client header는 사용자 인증이 아님. shutdown은 기존 worker를 drain하지만 이미 시작된 원격 provider 효과를 선점 취소하지 않음 |
| Studio | embedded local assets, DOM `textContent`, 외부 CDN 없음 | localStorage 최근 기록에 brief/답이 남을 수 있음. 프롬프트 미저장이라고 안내하면 안 됨 |
| MCP | 고정 protocol, body limit, stateless HTTP 설정, non-loopback bearer+Host allowlist, bind/Host에서 파생한 Origin allowlist를 rmcp validator에 적용 | Stateful session mode is disabled. HTTPS reverse-proxy Origin은 현재 public config가 표현하지 않음 |
| Codex | argv 배열, read-only sandbox 요청, event/thread/candidate 검사, PNG 검증·hash·count/placement 대조 | event 검사는 사후 관측. 알려지지 않은 동작을 나중에 거절해도 이미 생긴 외부 효과를 취소하지 못함 |
| 인증·프로세스 | API key는 사용하지 않으며 `forced_login_method="chatgpt"`로 구독 인증을 강제한다. Codex instruction은 private working directory의 stdin file로 전달; child 환경에서도 API key 제거 | host·외부 executable·계정은 별도 신뢰 경계. 외부 process 자체가 adapter가 제공한 입력을 읽는 것은 의도된 효과 경계 |
| 게시·저장 | 인접 temp와 단일 파일 hardlink/rename, 명시 overwrite | 전체 rollback·durable retry 보장 없음. PNG 게시 뒤 receipt/stdout 실패 가능 |
| 소스·공급망 | `unsafe_code = forbid`, 고정 toolchain/lockfile, offline build 경로 | lockfile과 source stamp만으로 실제 build/provider 성공을 증명하지 않음 |

MCP HTTP를 단순히 인터넷에 노출하지 않는다. 외부 노출은 명시 network policy와 TLS 종료 등 별도 운영 경계를 요구한다.
bearer는 `PROMPTGEN_MCP_BEARER_TOKEN` 환경 변수 사용을 권장하며 shell history·process argv·로그에 token을 남기지 않는다.
Host·Origin·인증은 서로 대체되지 않는다. 현재 Origin policy source는 bind/`allowed_hosts`이며 header 검증 mechanism은 rmcp validator 한 곳이다. HTTP admission과 MCP integration 결과는 [구현 현황](docs/IMPLEMENTATION_STATUS.md)의 최신 gate 기록을 따른다.

## 보장하지 않는 것

Codex 가용성·정책·인증, 모델의 완벽한 문구/기하/미적 재현,
PNG 구조 검증에 의한 시각 의미 증명, 모든 descendant process 종료, timeout 뒤 원격 미실행,
검증하지 않은 OS/browser 동작을 보장하지 않는다. 자동 재시도가 안전하다고 추정하지 않는다.

사용자 출력 삭제 위험은 [저장 운영](docs/RUNTIME_STORAGE.md), 나머지 수정·검증 조건은 [PLAN](docs/PLAN.md)을 따른다.

## 문제 보고

재현 입력, 명령, 환경, 실제 관찰, 영향 경로를 포함하되 token·credential·개인 이미지·`CODEX_HOME` 인증 파일은 제외한다.
실제 손상·게시·원격 요청 여부가 불확실하면 UNKNOWN으로 표시하고 성공/미실행으로 바꾸지 않는다.
