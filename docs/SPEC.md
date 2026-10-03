# promptGen 기능·수용 규격

## 규격의 범위와 원본

이 문서는 확정된 제품 의미와 관찰 가능한 수용 기준을 소유한다. 구현 상태·test 결과는 [IMPLEMENTATION_STATUS](IMPLEMENTATION_STATUS.md), 남은 결정은 [PLAN](PLAN.md)이다. schema 표현은 core Rust 생성 함수와 generated JSON을 연결해 관리한다. 버그에 맞춰 규범을 느슨하게 바꾸지 않는다.

| 계약 | 원본 |
|---|---|
| image 입력·진단·render 의미 | [ImagePromptRequest](../crates/promptgen-core/src/image/model.rs), [image validation](../crates/promptgen-core/src/image/validate.rs), [세부 image 규격](IMAGE_PROMPT_SPEC.md) |
| image/interview/compilation JSON Schema | [image](../schemas/image.schema.json), [interview](../schemas/interview.schema.json), [compilation](../schemas/compilation.schema.json), [생성 경로](../scripts/generate-schemas.sh) |
| screen raw input/handoff | [screen decoder/model](../crates/promptgen-core/src/screen/mod.rs), [사용 가이드](SCREEN_PROMPT_PLAYBOOK.md) |
| taxonomy | [image_catalog.json](../catalog/image_catalog.json), [catalog 규격](../catalog/README.md) |
| transport 표현 | [MCP](MCP.md), [Studio UX](IMAGE_STUDIO_UX_SPEC.md), [CLI 공개 surface](../crates/promptgen-cli/README.md) |

## 필수 기능

image 요청은 언어·목적·medium·taxonomy·scene·subjects·composition·lighting·color·constraints·output 및 선택 profile/reference/text/consistency 정보를 가진다. unknown field·잘못된 enum/range·충돌하는 role/change/preserve·지원하지 않는 category/profile 조합을 거절한다. C5의 `travel_journal`은 generate 또는 단일 base edit를 지원하며 제공 문구·사실과 원본 사진을 구분한다. 참조는 실제 전달 순서에 대응하는 role/index로 구분하며 설명에서 파일을 추측하지 않는다.

image는 `taxonomy.category`로 C1/C4/C5/C6/C10/C11 중 하나를 선택한다. taxonomy에는 결과 카테고리 하나만 둔다. 스타일·매체·조명·색·재질·구도는 독립 typed 필드다. interview는 선언한 answer key만 받고 알 수 없는 key를 거절한다. `app_icon`, `logo_identity`, `app_web_ui`, pose/storyboard는 [세부 image 규격](IMAGE_PROMPT_SPEC.md)을 따른다.

guided interview는 brief와 caller가 누적한 answers를 받아 빠진 결정을 질문한다. unknown/incompatible 답을 조용히 다른 값으로 승인하지 않는다. `needs_input`은 질문과 normalized 답을, `ready`는 사용 가능한 typed request와 compilation을, `invalid`는 진단을 제공한다. 서버 session에 의존하지 않는다.

screen은 `promptgen-screen-v1`의 목적·fidelity·theme·contentMode·viewport·raster·region·element와 선택 brief/bounds를 검증한다. placeholder/실제 명시 synthetic 문구, empty/loading/populated/no_results/error를 구분한다. 화면 이미지의 상태와 실제 앱 서비스 동작은 별개의 계약이다.

## 입력·출력·오류·전제조건

| surface | 입력·전제 | 출력·오류 |
|---|---|---|
| image compile | strict UTF-8 JSON 또는 public typed request | `CompilationOutcome`: `valid`/`valid_with_warnings`/`invalid` 의미, diagnostics와 metadata. error가 있으면 usable prompt 없음 |
| interview | image brief + answers | needs_input/ready/invalid. 질문 필요는 정상 분기이며 생성 완료가 아님 |
| screen compile | inclusive 128 KiB UTF-8 raw input, 유효한 구조/상태/좌표 | `promptgen-screen-handoff-v1`, status compiled, 원문 inputJson·inputSha256·prompt·promptSha256 또는 DecodeError |
| CLI | command/options/input 경로 또는 stdin, 명시 output 정책 | JSON/text/stdout/file; 0 정상, 2 usage/I/O, 3 입력/계약 거부, 4 provider 실행 오류. exit만으로 remote effect 여부를 추정하지 않음 |
| MCP | 지원 protocol 및 tool 입력/metadata | structuredContent/text/isError와 JSON-RPC/HTTP error를 구별 |
| Studio | image-only typed decision, `/api/v3` request, prompt-only 또는 명시 Codex 실행 | 질문/inspector/prompt 또는 Luna 검토 후 실제 artifact/실패. 현재 의도와 관련 없는 늦은 응답을 현재 결과로 표시하지 않아야 함 |

image render 길이 진단의 warning과 hard validation error를 구별한다. byte 한도와 character 한도는 서로 바꿔 쓰지 않는다. schema 자체의 유효성과 domain 교차 검증 통과도 구분한다.

screen viewport 각 축은 240–4096이며 raster는 1024×1024, 1536×1024, 1024×1536 중 하나다. region 1–12, element 1–64이며 bounds는 부모 범위 안에 있어야 한다. 명시 synthetic row는 최대 12행, 각 행 1–8개 셀로 동일 열 수를 갖고 table_header가 있으면 열 수가 맞아야 한다. primaryAction은 존재하는 enabled button을 참조한다. prompt는 128 KiB 이내다. 입력 version, handoff version, render 정책 version은 별개다.

## 선택 실행 기능

Codex 실행은 생성 또는 단일 base 편집을 수행하는 명시 mode다. 실행 전에 canonical prompt를 Codex CLI의 `gpt-5.6-luna`가 구조화 결과로 검토한다. 중요한 모순을 거절하면 이미지 provider를 호출하지 않는다. 승인 시 원본 prompt는 그대로 두고 제한된 시각 보강만 뒤에 추가하며, 실행 전 prompt와 원본·최종 해시를 receipt에 기록한다. 코드가 요청한 sandbox/event/후보·PNG/fidelity 계약을 만족한 결과만 게시해야 한다. 사진 LUT 등 prompt-level 의도를 PNG byte만으로 확인했다고 주장하지 않는다. 후보에 대한 모델 관측은 request의 count/placement/text와 프로그램으로 대조하되 독립적 시각 진실과 동일시하지 않는다.

이미지 실행은 ChatGPT 구독으로 인증된 Codex CLI 하나를 사용한다. API key는 요청하거나 사용하지 않는다. 생성은 generate/no-reference이며 편집은 `task_mode=edit`, reference 한 개/index=1/role=base와 실제 `--reference-image FILE`을 요구한다. 원본은 bounded regular PNG로 admission하고 private snapshot에 복사하여 hash를 고정한다. generation에는 그 snapshot을 `referenced_image_paths`로 명시하며, 시각 검수에는 원본 snapshot과 후보를 구분하여 첨부한다. 원본은 덮어쓰지 않는다. 실행은 `forced_login_method="chatgpt"`로 제한하고 API-key 환경 변수를 child에서 제거한다. 생성·편집 모두 같은 compilation/execution receipt wrapper를 반환한다. 납품 backend·디테일 의도와 실제 관찰한 모델을 구분하며 tool에서 관찰하지 못한 이미지 모델은 null로 기록한다.

`prompt-only`와 MCP compile은 계속 결정적·provider-free다. `luna-refine`은 이미지 생성 없이 검토 결과와 최종 prompt를 돌려주는 별도 CLI mode다. Studio의 Codex 실행에도 같은 Luna 검토가 적용된다. 이 검토 단계는 설치된 Codex CLI의 로그인 상태를 사용하고, 외부 provider 인증/실패 의미를 core에 넣지 않는다.

Web/MCP/screen/provider의 추가 조합, composite/pose 다중 reference 전송, batch/gallery/motion은 자동 필수가 아니다. 노출 범위를 넓힐 때 승인·계정·비용·원본·취소·결과 계약을 먼저 결정한다.

## 저장·권한·실패 의미

컴파일 계산은 파일·계정·네트워크 효과를 시작하지 않는다. provider는 명시된 reference·계정·목적지만 사용하고 prompt가 실행 권한을 새로 만들지 않는다. 사용자 원본과 기존 결과를 암묵적으로 덮어쓰지 않는다. 공개 `--force`/overwrite는 명시적 예외로 유지한다. 다른 output 역할이 같은 FS 대상을 가리키지 않도록 확인해야 한다.

이미지 publication과 receipt 전달, remote request와 local file commit은 단일 transaction으로 주장하지 않는다. 실패·timeout·불확실한 원격 결과를 성공 또는 미실행으로 바꾸지 않는다. 재시도/복구에 필요한 증거의 형식 확장은 별도 결정 대상이며 기존 contract가 보장하지 않는 멱등성을 가정하지 않는다. [저장 운영](RUNTIME_STORAGE.md), [보안](../SECURITY.md)을 따른다.

## 관찰 가능한 수용 기준

| 계약 | 수용 기준 |
|---|---|
| 결정적 compiler | 동일 의미/고정 정책의 요청은 같은 prompt를 얻고 invalid 입력은 유효 prompt로 사용되지 않는다. 원문 hash는 원문 bytes와 일치한다 |
| guided 사용자 결정 | 질문/답/추론이 구별되고 category 변경 시 부적합 답이 재사용되지 않는다 |
| screen 정확 전달 | 지정 상태·문구/행/참조/환산 경계를 보존하고 픽셀 정확성·실제 서비스 완성을 컴파일 결과로 주장하지 않는다 |
| provider admission | 실행 request/prompt, 검수한 후보, 게시 PNG, 반환 receipt의 identity가 서로 맞는다. 실패 후보를 성공 artifact로 내보내지 않는다 |
| 파일 보존 | 명시 overwrite 없이 기존 파일을 바꾸지 않으며 source/result/receipt 역할 충돌을 거절한다 |
| UI·protocol | stale completion은 현재 의도에 귀속되지 않는다. Studio가 receipt-bound artifact를 표시할 때 현재 bytes SHA가 receipt SHA와 달라지면 성공 이미지로 표시하지 않는다. protocol/header/auth/error contract를 충족한다 |
| 검증 증거 | source identity·실행 환경·명령·관찰 result의 적용 범위를 구분하고 mock/과거 기록을 현재 provider 성공으로 대체하지 않는다 |

## 시각 어휘·사진 편집

[최신 리뉴얼 근거](IMAGE_PROMPT_RESEARCH.md)의 23개 authoring token을 기존 typed 슬롯에 확장한다. prefix의 경쟁 축·명시 답과의 모순을 거절하며 provider에 slash 표기를 전달하지 않는다. 인터뷰 기본값은 `backend=codex-subscription`, `detail=high`다. detail은 렌더러가 프롬프트에 펼치는 시각 디테일 의도이며 engine 품질 제어가 아니다. `editorial_flat`, `graphic-design`, `schema compilation-outcome`은 지원하지 않는다.

원본 사진 brief는 edit로 분류하고 원본의 역할·보존·변경을 분리한다. 이미 명시한 보존·변경·문구는 재질문하지 않는다. 모호한 결정만 질문하며 실제 source 파일 전달은 실행 시 필수다. source edit의 인쇄 스타일은 추가 영역에만 적용한다. Studio의 참조 없는 실행 API는 이러한 edit를 Luna 전에 거절한다. Codex CLI가 실제 base PNG snapshot을 결합한다. 원본 바이트의 픽셀 동일성을 프롬프트만으로 증명하지 않는다.

문자 절개선 수는 `text_elements[].separator_count`(각 줄, 0–8)로 선언한다. 간격을 요구하면서 숫자가 없으면 입력을 거절한다. 간격 관찰은 원하는 숫자 없이 후보의 문자 영역만 사용하고, 실제 관찰값을 로컬 코드가 비교하여 전체 model pass보다 우선한다. macOS 문자 영역 crop은 검수용 파생 자료이며 게시 후보를 수정하지 않는다.
