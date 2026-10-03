# promptGen 현재 구현 지도

version 4.0.0. 실제 source revision은 [REVISION](REVISION), 검증 기록은 [VERIFICATION.json](VERIFICATION.json)이다. `.git`이 없어 commit·branch·변경 diff는 알 수 없다. 과거 archive는 evidence이며 활성 runtime의 legacy 계약이 아니다.

## 유일한 의미와 효과 owner

| 경계 | 입력 → 계산/효과 → 출력 |
|---|---|
| core image | strict typed request → 교차 검증·조건을 자르지 않는 render → compilation·diagnostics·plan |
| core interview | brief + caller answers → 시각 어휘 확장·필요 결정·typed compile → needs_input/ready/invalid |
| core screen | strict raw JSON → 상태·기하·행 검증·hash → handoff |
| CLI | command/stdin/file → core·선택 adapter 조립·목적지 admission → stdout/result/exit |
| Studio | browser intent → stateless interview·명시 no-reference generation → prompt/artifact/receipt |
| MCP | stdio/HTTP → core read-only 도구 → structured response |
| Codex | compilation·Luna proof·선택 base PNG → 구독 인증·snapshot·process/event/candidate 검사·PNG 게시 → receipt |

core 컴파일은 provider 효과를 시작하지 않는다. core PNG path utility에는 파일 읽기가 있다. browser는 explicit answers·normalized answers·pending request identity를 분리한다. 서버가 사용자 인터뷰를 영속 저장하지 않는다.

## 현재 단일 계약

- `editorial_flat` enum·renderer·길이 절단·300–550자 별도 예산을 제거했다. 일반 작업은 하나의 구조화 renderer로 조건을 보존한다.
- `graphic-design` 입력 alias와 `schema compilation-outcome` alias를 제거했다. 정본은 `graphic_design`, `schema compilation`이다.
- image Studio의 단일 kind tab·cross-kind drafts·journey navigation·중복 category filter/quick selector·density·선택 tray·가짜 prompt 확정 동작을 제거했다.
- 결과 프롬프트는 기본 작업 면에 두고 설정·검수·JSON 상세는 필요할 때 연다. 기존 user data·catalog PNG·historical evidence·legal files는 삭제하지 않았다.
- 기존 image model 계약은 종료했다. 정본 `IMAGE_BACKEND`은 `codex-subscription` 하나다. API 모델·품질 옵션을 runtime 설정으로 노출하지 않는다.

## 핵심 신규 연결

시각 어휘는 기존 domain 슬롯으로 변환한다. 별도 magic command나 병렬 prompt section은 없다. C5 `travel_journal`은 일반 광고 directive와 분리된 사진·종이·문구 의미를 갖는다. source edit는 원본 내부의 빛·색·정체성과 추가 영역을 구분한다. 실제 source bytes 전달은 Codex 구독 adapter가 소유한다.

실제 외부 원격 취소·멱등성·픽셀 동일성·최고 시각 품질은 선언으로 보장하지 않는다. runtime 및 gate의 현재 결과는 [Current](docs/IMPLEMENTATION_STATUS.md)에만 기록한다.


API key를 요청하거나 사용하지 않는다. 생성·편집 모두 로그인된 Codex CLI의 ChatGPT 구독을 사용하며 인증 방법을 명시적으로 제한한다.
