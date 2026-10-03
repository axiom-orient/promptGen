# 포즈 전이 프롬프트 계약

## 역할과 입력

`render_profile=pose_transfer`는 참조 포즈와 캐릭터 외형의 권위를 나누는 workflow profile이다.
새 taxonomy ID가 아니며 C1/C4/C5/C6/C10/C11은 여전히 최종 결과물의 종류를 뜻한다.
정본 예제는 [image-pose-transfer.json](../examples/image-pose-transfer.json)이다.

`task_mode=composite`, 순서가 있는 참조 정확히 두 개, 결과 subject 하나/count=1이 필요하다.
첫 `role=pose`는 보이는 전신 기하·신체 기준 좌우·발 접촉·무게·관절·손발 형태·몸/머리 방향·시선·카메라·원근·겹침/가림을 소유한다.
둘째 `role=subject`는 캐릭터 정체성·얼굴·기본 표정·비율·머리카락·의상·색·재질·선·render style을 소유한다.
`change_only`에는 외형/style 전이를, `preserve`에는 두 참조의 불변조건과 output lock을 명시한다.

## 충돌 규칙

문장과 보이는 포즈가 충돌하면 **포즈의 가시 기하**가 우선한다. 그 밖의 scene/output/safety typed 계약은 유지한다.
신체 기준 좌우를 관찰자 좌우로 바꾸거나 mirror하지 않는다. 가까운 팔다리·보이는 몸쪽·단축·겹침은 참조 카메라를 따르며 정면으로 정규화하지 않는다.
subject는 눈·얼굴·기본 표정의 권위다. pose에서 가져오는 것은 머리 회전과 시선 방향이다.
숨은 관절을 보기 좋은 정석 동작으로 발명하지 않고 silhouette와 occlusion을 보존한다.
포즈 인물의 얼굴/의상을 가져오거나 캐릭터 시트의 기본 자세/시선으로 되돌리지 않는다.

## 렌더 순서

목적/task → 참조 순서와 역할 → change/preserve → pose fidelity → outcome directives → scene/subject/composition/camera/light/color/constraints/output.
pose fidelity heading은 compiler가 소유한다. 사용자 문자열은 데이터로 인용하며 이 section이나 lock을 덮어쓰지 않는다.

## 검수 gate

| 순서 | 관찰 기준 | 실패 시 수정 범위 |
|---|---|---|
| Gate 1 · 전체 동작 | 양발 위치·접촉·각도·간격, stance 폭/깊이, 지지 다리·앞뒤 무게, 골반/몸통 회전·기울기, 전신 scale·중심·동작 시점 | 관절 사슬·stance/중심·silhouette부터 수정. 얼굴 polish로 실패를 덮지 않음 |
| Gate 2 · 팔다리·손발 | 어깨→팔꿈치→손목→손, 골반→무릎→발목→발, 실행/지지 역할, 손바닥/손등·주먹/펴짐, 발끝·무릎 정렬·겹침·단축 | 틀린 사슬/끝 형태만 수정하고 통과한 stance/silhouette 보존 |
| Gate 3 · 방향·시선 | 몸 각도·보이는 쪽, 머리 회전/기울기, 눈 시선, 카메라·가까운 팔다리 | mirror나 관절 변경 없이 방향/시선 수정 |

앞 gate의 실패를 뒤 gate 합격으로 덮지 않는다. identity는 매 gate에서 함께 검사한다.
얼굴·기본 표정·비율·의상·색·스타일을 유지하고 pose 수정은 identity를, identity 수정은 이미 맞는 pose를 오염시키지 않는다.

## 작성·수정 기준

준비/타격/완료/복귀 시점, 캐릭터 기준 앞발·실행 손과 좌우, 지지/무게, 발 간격·깊이·발끝,
관절 사슬·손 모양·목표 높이, 몸/머리/시선/가까운 팔다리, framing·전신 포함·scale·camera/output을 관찰 가능한 말로 적는다.
기술 이름이나 “같은 포즈/역동적/옆을 보기”만으로 정렬을 대체하지 않는다.

가장 이른 실패 gate, 관찰된 차이, 한 번의 수정, 유지할 lock을 명시한다.
동작/stance → 사슬과 silhouette, 국소 손발 → 해당 detail, 방향/시선 → camera와 신체 좌우,
외형 drift → subject lock 순으로 다루며 “더 비슷하게”라는 전체 재작성으로 통과 조건을 잃지 않는다.

## 실행과 검증 경계

```sh
promptgen image --input examples/image-pose-transfer.json --mode prompt-only --format text
cargo test -p promptgen-core image:: --locked --offline
cargo test -p promptgen-cli --test cli --locked --offline
python3 scripts/audit-prompts.py --check
```

prompt에 로컬 파일 경로를 적는 것은 이미지 첨부가 아니다. 실제 두 참조를 전달하고 순서·identity·결과를 증명하는 adapter가 필요하다.
첨부 제품의 Codex 및 직접 Codex 단일 base edit 범위는 [Current](IMPLEMENTATION_STATUS.md)에 구별되어 있으며,
이 profile의 컴파일 계약만으로 참조 생성 성공을 약속하지 않는다. gate는 수용 기준이지 모든 provider에 배선된 독립 시각 검사기의 존재 주장이 아니다.
