# ADR 002 · screen의 원문·상태·기하 전달

## 상태

**기존 채택 결정의 재구성.** 기존 연구 기록일 2026-09-08, 최초 채택일 UNKNOWN,
이번 문서 통합일 2026-09-13이다. 새로운 host 연동 승인이나 이미지 품질 개선의 실증 보고가 아니다.
[screen source](../../crates/promptgen-core/src/screen/mod.rs), [renderer](../../crates/promptgen-core/src/screen/render.rs),
[tests](../../crates/promptgen-core/src/screen/tests.rs), [13개 예제](../../examples/screens/)가 구현된 경계를 보여 준다.

## 맥락

화면 prompt에는 실제 사용자/작업/정보 밀도/주요 행동, 문구와 상태, 기하가 필요하다.
설계 설명·PNG 외형·작동하는 서비스·접근성 인증은 다른 결과다.
논리 좌표를 나열했다는 사실이 모델의 pixel 정확도를 보장하지 않는다.

## 결정

flat-canvas screen 요청은 purpose와 선택적 audience/userTask/density/primaryAction을 보존한다.
region/element의 포함 관계와 활성 button 참조를 검증하고, 실제 문구·행/셀·Unicode·줄바꿈을 유지한다.
placeholder, empty, loading, no_results, error, populated를 분리한다. 부분 오류로 정상 영역을 지우지 않는다.

design 좌표는 양 축 raster의 경계로 결정적으로 환산하고 인접 경계를 같은 방식으로 반올림한다.
입력 schema·handoff schema와 `SCREEN DESIGN CONTRACT v2` prompt 정책을 구분한다.
원문 inputJson과 input/prompt byte hash를 전달하며 컴파일 계산은 provider 효과·승인·retry를 소유하지 않는다.

## 결과

동일 원문/정책의 재컴파일과 요소별 검수 기준을 제공한다. PNG가 정확한 위치/상태/문구를 구현했는지는 실제 artifact에서 별도 측정한다.
새 provider·학습 모델·A2UI 통합·외부 UpAgent API를 채택한 결정이 아니다.
이전 문서에 있던 host 경로·지역적 폭 측정 수치의 원본 자료는 첨부에 없어 [PG-008](../../ANALYSIS.md#pg-008)의 UNVERIFIED로 분리했다.
수치 이력을 규범이나 성공 claim으로 계속 복제하지 않는다.

## 기존 참고문헌

아래는 2026-09-08 문서가 기록한 참고문헌/버전이며 **이번의 재검증 완료 목록이 아니다**.
연구의 성능을 이 제품의 이미지 성능으로 이전하지 않는다.

| 참고문헌 | 기존 기록의 적용 맥락 |
|---|---|
| [OpenAI image guide](https://developers.openai.com/cookbook/examples/multimodal/image-gen-models-prompting-guide) | 구획·목적·변경/보존, flat canvas와 맞지 않는 device frame은 비채택 |
| [OpenAI frontend guide](https://developers.openai.com/api/docs/guides/frontend-prompt) | 사용자·작업·밀도; 실제 frontend 동작을 PNG 약속으로 바꾸지 않음 |
| [Declarative UI paper v1](https://arxiv.org/html/2609.04184v1) | 지원 role/catalog·참조 경계; 학습 성능은 비이전 |
| [Design Theater v2](https://arxiv.org/html/2607.22928v2) | 설명과 실제 결과를 구별 |
| [Design2Code](https://github.com/NoviScl/Design2Code) | 텍스트·위치·요소별 비교, 기초 연구 |
| [A2UI v1.0](https://github.com/a2ui-project/a2ui/blob/main/specification/v1_0/docs/a2ui_protocol.md), [catalog](https://raw.githubusercontent.com/a2ui-project/a2ui/main/specification/v1_0/catalogs/basic/catalog.json) | 명시 포함 관계·참조 validation; 연동 채택 아님 |
| [Carbon empty](https://carbondesignsystem.com/patterns/empty-states-pattern/), [loading](https://carbondesignsystem.com/patterns/loading-pattern/) | 데이터 부재·결과 없음·대기 구별 |
| [GOV.UK errors](https://design-system.service.gov.uk/components/error-message/) | 구체 오류·입력 보존·부분 실패 |
