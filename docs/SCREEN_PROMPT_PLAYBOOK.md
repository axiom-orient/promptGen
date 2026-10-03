# 화면·서비스 프롬프트 작성 가이드

화면은 특정 사용자가 특정 작업을 하는 순간으로 작성한다. `screen` compiler는 typed 요청을 결정적 이미지 prompt로 만든다.
실행·승인·재시도는 실제 소비 host의 책임이며 서비스처럼 보이는 PNG와 작동하는 서비스는 별도 결과다.
외부 UpAgent의 API·배선·측정 자료는 이 첨부 범위에서 확인되지 않았다. [Current](IMPLEMENTATION_STATUS.md)의 외부 경계를 따른다.

## 바로 사용하기

저장소 루트에서 실행한다. 컴파일에는 계정/provider가 필요 없다. text는 prompt만, JSON은 원문과 hash를 포함한 handoff다.

```sh
cargo run -p promptgen-cli -- screen --input examples/screens/sign-in.json --format text
cargo run -p promptgen-cli -- screen --input examples/screens/mixed-state.json --format json
```

`--result <새 파일 경로>`로 저장하며 기존 파일 overwrite는 기본 거절된다.
일반 image Studio/MCP는 이 screen 계약을 받지 않는다. host는 원본 JSON·prompt bytes·각 hash를 정확히 연결하고 임의 요약을 새 컴파일 결과로 취급하지 않는다.
이 지침은 특정 외부 host API가 이미 연결됐다는 주장이 아니다.

## 입력을 만드는 순서

1. `purpose`: 서비스 종류, 현재 페이지와 작업을 적습니다. “아름답고 혁신적인 대시보드”보다
   “배송 담당자가 주문을 비교하고 실패한 배송 조회를 재시도하는 주문 화면”이 구체적입니다.
2. 선택적 `brief`: `audience`, `userTask`, `primaryAction`, `density`를 모두 제공합니다.
   `primaryAction`은 실제 활성 `button` 요소 ID입니다. 임의 문구나 비활성 버튼은 거부합니다.
   `density`는 `compact` 또는 `comfortable`입니다. 전자는 운영·비교 작업, 후자는 집중 입력에 적합합니다.
3. `viewport`는 논리적 디자인 좌표, `raster`는 출력 크기입니다. 가능하면 둘의 종횡비를 맞춥니다.
   현재 허용 raster는 1024×1024, 1536×1024, 1024×1536입니다. 모델 일반 사양과 이 어댑터 허용 범위는 다릅니다.
4. `regions`로 화면을 나눕니다. `bounds`는 뷰포트 기준입니다. 영역별 `dataState`, `stateText`, `rows`를 정합니다.
5. `elements`에 실제 문구와 역할을 넣습니다. 중요 요소에는 선택적 `bounds`를 지정합니다.
   이는 부모 기준 상대좌표가 아니라 **뷰포트 절대좌표**이며 부모 영역 안에 있어야 합니다.
6. 마지막으로 `fidelity`와 5개 theme 토큰을 정합니다. 분위기 형용사로 구조 결정을 대체하지 않습니다.

선택 입력의 형태:

```json
{
  "brief": {
    "audience": "배송 담당자",
    "userTask": "주문을 확인하고 배송 정보를 다시 조회한다",
    "primaryAction": "retry",
    "density": "compact"
  }
}
```

이 조각만으로는 완전한 요청이 아닙니다. 아래 예제를 복사하고 필요한 값을 수정하세요.
명시하지 않은 요소 위치는 읽기 순서를 따르는 배치 힌트이며, 픽셀 단위 배치 계약이 아닙니다.
`brief`를 생략하면 기존 요청이 유지되며 주요 행동을 임의로 추측하지 않습니다.

## 상태와 예제

| 작성 목적 | 예제 | 핵심 |
|---|---|---|
| 텍스트 없는 배치 초안 | [placeholder](../examples/screens/placeholder.json) | 정적 블록. 로딩·가짜 글자·빈 상태 문구 금지 |
| 실제 문구가 있는 저충실도 화면 | [wireframe-populated](../examples/screens/wireframe-populated.json) | 회색 구조 + 정확한 데이터 |
| 첫 데이터 생성 전 | [styled-empty](../examples/screens/styled-empty.json) | 데이터 없음과 다음 행동. 임의 일러스트·중복 버튼 금지 |
| 데이터가 있는 서비스 | [styled-populated](../examples/screens/styled-populated.json) | 행 개수·셀 연결·정렬 보존 |
| 데이터 대기 | [loading](../examples/screens/loading.json) | 데이터 영역만 skeleton. 내비게이션과 라벨 유지 |
| 필터 결과 없음 | [no-results](../examples/screens/no-results.json) | 처음 사용하는 빈 상태와 구별, 검색·복구 유지 |
| 전체 영역 오류 | [error](../examples/screens/error.json) | 정확한 오류 문구, 성공·가짜 데이터 금지 |
| 좁은 화면 | [mobile](../examples/screens/mobile.json) | 모바일 좌표의 별도 요청. 데스크톱 축소를 반응형이라 부르지 않음 |
| 로그인 | [sign-in](../examples/screens/sign-in.json) | 주요 행동과 핵심 입력의 명시적 bounds |
| 한국어 폼 오류 | [form-error-ko](../examples/screens/form-error-ko.json) | Unicode·입력값 유지, 오류 범위 한정 |
| 설정 | [settings](../examples/screens/settings.json) | 현재 값과 저장 행동, 기능 설명 광고 금지 |
| 상품 관리 | [catalog](../examples/screens/catalog.json) | 상품·재고·가격 연결을 유지하는 비교 화면 |
| 일부만 실패 | [mixed-state](../examples/screens/mixed-state.json) | 정상 주문 데이터와 실패한 배송 영역 공존 |

`empty`와 빈 `stateText`는 내비게이션·폼처럼 레코드가 없는 영역에도 사용합니다.
`no_results`는 명시적 안내가 필요합니다. `populated`는 실제 전달한 합성 행이 필요합니다.
현재 데이터 모델은 최대 12개 영역, 64개 요소, 영역당 12행·행당 8셀입니다.
이미지·차트·토글 등의 새 역할은 아직 지원하지 않습니다. 지원하지 않는 역할을 문자열로 꾸미지 마세요.


## 프롬프트 작성·검수 규칙

좌표는 논리 viewport를 raster 양 축으로 환산한 경계로 전달하며 인접 영역은 같은 반올림 경계를 사용한다.
prompt에 적힌 경계 수치와 실제 PNG의 측정값은 별개다. compiler가 계산한 위치를 생성기가 지킨다고 가정하지 않는다.

빈 상태에 요청하지 않은 그림·중복 행동을 추가하지 않는다. Unicode·줄바꿈·행별 셀을 보존한다.
`table_header`의 `|`는 셀 구분이며 출력 글자로 요구하지 않는다.
수정은 가장 이른 관찰 결함 하나에 집중하고 이미 맞는 구조·문구·상태는 보존한다.
단순 resize를 내부 layout 수정으로 부르지 않는다. 검수 기준을 생성 전에 고정하고 실패·불확실한 결과를 숨기지 않는다.

## 평가·재컴파일

동일 input/raster/provider 설정에서 input SHA·prompt SHA·artifact SHA·실제 dimensions·영역 위치·누락 요소·문구/셀 오류·상태를 별도로 기록한다.
다른 모델이나 다른 입력을 통제된 A/B로 부르지 않으며 화면 인상만으로 정확성을 승인하지 않는다.
기존 보고의 지역적 수치 관측은 원본 artifact·측정 방법을 확인할 때만 재사용한다. 첨부에 없는 과거 측정을 현재 성공/실패 증거로 승격하지 않는다.

입력 `promptgen-screen-v1`, handoff `promptgen-screen-handoff-v1`, prompt 정책 `SCREEN DESIGN CONTRACT v2`를 구별한다.
정책이 바뀌면 원본을 다시 컴파일하고 새 prompt hash를 사용한다. 과거 handoff를 새 결과로 가장하는 호환 경로는 제공하지 않는다.
13개 예제의 존재와 현재 실행 통과는 다르며 실제 검사 범위는 [Current](IMPLEMENTATION_STATUS.md)가 소유한다.
설계 맥락은 [ADR 002](adr/002-screen-contract.md), 미검증 과거 host/측정 자료는 [PG-008](../ANALYSIS.md#pg-008)을 참조한다.
