# promptgen-web 분석

## 책임과 실행 경계

promptgen-web은 image-only Studio와 local HTTP server다. core가 의미와 검증을 소유하고, Web은 network admission, browser state, Codex 호출 조정, catalog/artifact bytes 반환을 소유한다. screen compile과 사진 업로드 transport는 이 경계에 없다.

`serve_listener`는 실제 bind 주소로 설정을 다시 검증한다. 요청에는 body/header/time/Host/Origin limit가 적용된다. catalog JSON은 compiled source와 일치해야 하고 각 card bytes는 embedded manifest의 길이·SHA와 일치해야 한다. generation request는 core interview/compile 검증 뒤에 Luna review와 Codex imagegen으로 전달된다.

## 단일 공개 HTTP 계약

| 목적 | 현재 경로·조건 |
|---|---|
| Studio/static assets | `/`, `/index.html`, `/app.css`, `/app.js` |
| health/catalog/LUT | `/api/v3/health`, `/api/v3/catalog`, `/api/v3/lut-presets` |
| catalog card | `/api/v3/catalog/assets/{id}` |
| interview/generation | `/api/v3/interview`, `/api/v3/generate` |
| browser client identity | `X-PromptGen-Client: web-v3` |
| generated artifact | `/artifacts/{name}?sha256={64 hex}` |

다른 API version은 route가 없다. artifact SHA query가 빠지거나 중복·변형되면 400, 파일의 현재 SHA가 receipt digest와 다르면 409, query가 정확하고 bytes가 일치할 때만 PNG를 반환한다.

## 상태와 publication

`invalidatePendingWork()`가 brief/category/answer/reset/recent restore 시 interview·generation request identity를 함께 증가시킨다. 늦게 도착한 결과는 현재 identity가 아닐 때 state에 반영되지 않는다. local history는 `promptgen.recent.v5` 한 key만 쓴다.

서버 종료는 새 accept를 중지하고 이미 수락한 local worker를 join한다. 이미 시작된 원격 Codex 요청을 강제로 취소하지는 않는다. generation receipt SHA는 UI artifact URL에 결속되고 서버는 현재 bytes를 다시 hash한다.

## 현재 리스크와 검증

- HTTP/API, embedded assets, catalog admission, artifact digest 검증은 `verify.sh` 안의 release smoke와 Rust tests로 확인한다.
- 실제 browser interaction, keyboard/screen reader와 visual layout은 별도 browser session에서 아직 확인하지 않았다.
- Codex imagegen remote effect·candidate visual fidelity는 실제 이미지 실행까지 확인해야 한다.
- 원본 PNG 편집은 CLI/Codex adapter가 담당하며 Studio는 사진 업로드를 제공하지 않는다.

최신 검사 결과는 [구현 현황](../../docs/IMPLEMENTATION_STATUS.md), 남은 browser/provider 검증은 [PLAN](../../docs/PLAN.md)에 있다.

