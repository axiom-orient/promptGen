# 이미지 프롬프트 상세 계약

## 1. 권위·taxonomy

`ImagePromptRequest`와 core의 typed validation/render가 입력 의미를 소유한다.
catalog directive는 결과물을 구체화하되 task mode·reference·exact text·dimension·detail·safety를 덮어쓰지 않는다.
`taxonomy`는 C1/C4/C5/C6/C10/C11 중 하나인 `category` 하나만 가진다.
medium·composition·lighting·palette·surface·text·reference는 독립 typed 필드다.
[model](../crates/promptgen-core/src/image/model.rs), [validation](../crates/promptgen-core/src/image/validate.rs),
[schema](../schemas/image.schema.json)를 함께 참조한다. schema의 구조 통과만으로 교차 검증이 끝나지 않는다.

## 2. task·reference·변경 의미

`generate`는 원하는 결과를 직접 기술하고 edit-only `change_only`를 쓰지 않는다.
`edit`는 기준 이미지 하나의 변경/보존 범위를 명시한다. `composite`는 둘 이상의 참조가 소유하는 역할과 결과의 기준을 명확히 한다.
참조 index는 1부터 실제 전달 순서대로 연속이어야 하며 `description`과 중복 없는 `use_for`로 책임을 적는다.
같은 지시를 `change_only`와 `preserve`에 동시에 넣지 않는다.

`pose_transfer`는 composite mode에서 pose/subject 두 역할의 권위를 분리한다. 일반 composite도 base를 최대 하나까지 허용하고 layout reference가 결과를 고정할 수 있다. generate는 subject/style/product/layout/palette reference를 사용할 수 있고 base/mask role은 허용하지 않는다. edit는 base 하나를 요구한다. 각 mode의 전체 제약은 [validator](../crates/promptgen-core/src/image/validate.rs)가 소유한다. provider별 실행 가능한 subset은 core가 표현하는 요청과 별개다.

## 3. profile·연속성

`pose_transfer`는 composite workflow이며 일곱 번째 outcome이 아니다.
pose는 보이는 신체 기하·방향·시선·camera/occlusion, subject는 identity·얼굴·기본 표정·비율·머리/의상/style을 소유한다.
[포즈 계약](POSE_TRANSFER_PROMPT_SPEC.md)의 순서와 gate를 따른다.

`cinematic_storyboard`는 C10의 generate/illustration profile이며 reference·image text 없이
고정 layout·screen direction·identity anchor·순서 있는 camera/action/gaze panel을 사용한다.
[스토리보드 계약](CINEMATIC_STORYBOARD_PROFILE.md)을 따른다.

`consistency`의 `multi_view`, `character`, `temporal`, `semantic`은 관찰할 요구 축이지 모델 보장이 아니다.
storyboard는 character/temporal/semantic 및 panel ID와 일치하는 sequence를 요구한다.
[연속성 계약](MULTI_IMAGE_CONSISTENCY.md)은 공유 anchor와 이전 결과 조건화의 의미를 소유한다.

## 4. 앱 아이콘·무문구 UI

`profile=app_icon`은 C4 generate, non-photo(`graphic_design`/`illustration`/`3d`/`mixed`),
중앙 subject 하나/count=1, 빈 `text_elements`, opaque PNG **1024×1024**다.
제품 목적 → 핵심 은유/구성 기하 → 단순 2–3 시각 plane의 중앙 composition → 32px silhouette 검토 순으로 지시한다.
문자·숫자·가짜 글씨·UI screenshot·device mockup·rounded-square mask·icon frame·watermark는 금지한다.
단일 master concept이며 SVG/vector/editable layer/platform package를 약속하지 않는다.

`app_web_ui`에서 exact text가 비어 있으면 app/web/agent 화면의 구조·navigation·state·component 관계만 표현한다.
title/label/data/chart typography를 발명하지 않으며 읽히는 문구·숫자·가짜 글씨·임의 data/chart를 배제한다.
exact text가 제공되면 기존 text-directed 계약을 유지한다.

## 5. 렌더·문구 순서

compiler-owned section은 reference → change/preserve → 해당 pose/storyboard/consistency → scene → subjects → composition/space → lighting/color/surfaces/exact text/constraints/output의 책임 순서를 따른다.
사용자 문자열은 데이터로 인용하며 section 권위를 만들지 않는다. profile 전용 목적/directive의 세부 위치는 [renderer](../crates/promptgen-core/src/image/render.rs)가 정한다.

문구는 기계 label이 아니라 순서 있는 인용 행으로 전달한다.

```text
1. "URBAN"
2. "SIGNAL"
```

각 인용 행의 철자·Unicode 공백·행 순서·placement·글꼴 계열·size·color·선택 spelling hint를 구별한다. 원문 행 안의 공백과 줄 사이 경계는 컴파일 중 바꾸지 않는다.
절개선·분리 간격을 요구하는 text treatment는 `separator_count`(0–8)를 각 줄의 정본 숫자로 선언한다. 3개 간격은 4개 조각과 다르다. 실행 검수는 원하는 숫자를 보여주지 않는 문자 영역 관찰에서 실제 간격 수를 읽고 로컬 코드가 선언값과 비교한다.
무문구 요청은 읽히는 text·logo·watermark를 제외한다. 사용자가 제공하지 않은 문구를 자연스럽다는 이유로 추가하지 않는다.

## 6. 기하·hard constraint·출력

count·panel 수·identity anchor·placement·배제 조건은 subjects/composition/`constraints.required_elements`로 관찰 가능하게 적는다.
복수 instance의 placement는 **각 보고된 중심이 들어가야 하는 group 영역**이지 pixel-perfect packing 주장이 아니다.
수·scale·간격·pose에 맞는 영역을 주고 배열 자체가 중요하면 “느슨한 3×3”처럼 별도로 명시한다.
고밀도 C5/C6/C10/C11의 디테일 의도 판단은 [interview 수용 기준](IMAGE_INTERVIEW_SCENARIOS.md)을 따른다.

납품 output 계약은 `backend=codex-subscription`, `detail=auto|low|medium|high`, PNG, opaque와 선언한 ratio/dimensions다. detail은 프롬프트의 시각 묘사 수준이다. 구독 도구에는 model·quality·size 인자가 없으므로 native 설정을 강제했다고 주장하지 않는다.
차원은 양수·16의 배수, 각 변 최대 3840, 총 pixel 655,360–8,294,400, 종횡비 1:3–3:1이다.
고해상도 경고와 hard 거부를 구별한다. prompt를 렌더했다는 사실은 opaque·정확 문구·count·pixel geometry가 실제 이미지에서 검증됐다는 뜻이 아니다.

## 7. 실행·증거 계약

Codex는 ChatGPT 구독으로 generate/no-reference 또는 edit/single-base를 수행한다. compiled prompt identity·backend·검수한 candidate bytes·게시한 PNG·receipt가 일치해야 한다.
모델이 보고한 count/center와 typed request를 프로그램으로 대조하며 placement 관측 tolerance는 2 percentage points다.
모델의 pass만으로 이 비교를 덮지 않는다. event 검사, PNG 구조 검사, 시각 검수, local publication은 서로 다른 검증이다.

단일 base 편집은 `task_mode=edit`, reference 한 개/index=1/role=base와 실제 `--reference-image FILE`로 한정한다.
원본 PNG를 private snapshot으로 고정하고 `referenced_image_paths`로 실제 편집 도구에 전달한다. 그 SHA를 receipt에 결합한다. API key를 요청하거나 사용하지 않는다.
검수에는 원본과 후보를 함께 첨부하고 보존·변경을 비교한다. composite/pose/multiple-reference/mask 및 Web/MCP의 reference 전송은 지원하지 않는다.

receipt hash 필드의 존재만으로 후보/게시 결합·원격 성공·지속 저장을 증명하지 않는다.
공개 형식과 현재 연결/충족 범위는 [SPEC](SPEC.md), [Current](IMPLEMENTATION_STATUS.md),
[Codex 분석](../crates/promptgen-codex/ANALYSIS.md)에 있다.

## 8. 검증 명령

```sh
cargo test -p promptgen-core --locked --offline
cargo test -p promptgen-codex --locked --offline
python3 scripts/renew-image-catalog.py --check
python3 scripts/audit-prompts.py --check
```

## 여행·다이어리 프로필

C5 `travel_journal`은 주 장면/원본 사진, 종이·메모, 정확 문구와 제공 사실을 분리한다. 일반 광고 CTA·브랜드 directive를 적용하지 않는다. generate 또는 단일 base edit를 지원한다. edit에서 추가된 표현은 선언된 추가 영역에만 적용하며 사진 내부의 조명·색·정체성은 보존한다. 실제 bytes binding은 Codex 구독 adapter가 담당한다. 사용자가 제공하지 않은 지명·날짜·주소·이동시간·장식 글자를 발명하지 않는다.
