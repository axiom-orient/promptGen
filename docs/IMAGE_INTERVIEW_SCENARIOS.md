# 이미지 interview 수용 시나리오

이 표는 taxonomy v5의 대표 route와 회귀 **요구**다. 생성 이미지의 실측 결과나 테스트 실행 기록이 아니다.

| Case | Brief | Route | 먼저 해결할 결정 |
|---|---|---|---|
| C1-01 | 30대 가구 디자이너가 작업실에서 의자를 스케치하는 세로 화보 | C1 | 구체적 의상 |
| C4-01 | 무광 알루미늄 스피커 1개와 재생지 패키지 1개 제품 사진 | C4 | 빠진 제품·재료 설명 |
| C4-02 | 개인 기억 앱 아이콘, 별 안의 접힌 종이 핵심 은유 | C4 + `app_icon` | 제품 목적·중앙 은유 하나·32px silhouette |
| C5-01 | 시트러스 음료 출시 한국어 포스터, 캔 1개 | C5 | 요청한 exact copy 또는 campaign 계획 |
| C6-01 | 커피 생산 6단계 인포그래픽 | C6 | 정확한 여섯 단계·읽기 순서 |
| C10-01 | 붉은 목도리 여우의 4패널 이야기 | C10 | 네 장면·identity anchor |
| C11-01 | 고대 수몰 도시에 도착한 탐험가 키아트 | C11 | 세계의 초점·scale |

## route·상태 회귀

| Case | 이전 상태 | 입력/행동 | 요구 결과 |
|---|---|---|---|
| STATE-01 | C1 의상 답변 완료 | C5로 변경 | wardrobe/face/hair/safety 답 잔존 없음 |
| STATE-02 | 느린 C1 분석 | 더 새로운 C5 분석 | 늦은 C1 응답 무시 |
| STATE-03 | LUT 있는 사진 | illustration으로 변경 | `photo_lut` 제거 |
| STATE-04 | 알 수 없는 answer key | JSON 해석 | 구체 경로의 입력 오류로 거절, 값을 무시하거나 바꾸지 않음 |
| STATE-05 | 잘못된 category | 분석 | route별 추론 전에 category 재질문 |
| STATE-06 | exact text 요청 | 두 줄 제공 | 순서대로 인용한 두 줄; `exact_text_line_*`를 이미지 문구로 사용하지 않음 |

## 관찰 가능한 hard constraint

C4는 제품 하나·package 하나의 수를 보존한다. app icon은 중앙 subject 하나/count=1/무문구/불투명 1024² PNG다.
C5는 hero와 명시 supporting-object 수를, C6는 각 step/data point 한 번씩을, C10은 panel 수와 같은 identity anchor를 보존해야 한다.
C11은 brief가 달리 지정하지 않으면 주인공 하나다. 부정 조건을 긍정 subject/medium/light/text로 추론하지 않는다.
이 조건을 prompt에 넣는 것과 이미지가 실제 충족하는 것은 별도로 검수한다.

## 출력 기본값

| ID | Ratio | high detail 판단 맥락 |
|---|---|---|
| C1 | 2:3 | exact text 또는 명시 high-detail |
| C4 | 3:2 | exact text 또는 명시 high-detail |
| C4 + `app_icon` | 1:1 | 제품 은유·32px silhouette |
| C5 | 2:3 | campaign hierarchy·exact text 가능성 |
| C6 | 16:9 | 높은 정보 밀도·읽기 순서 |
| C10 | 2:3 | 여러 panel의 identity·layout |
| C11 | 16:9 | 인물/환경의 scale·분위기 |

수치와 자동 추론의 원본은 [interview](../crates/promptgen-core/src/interview/image.rs)이며,
현재 검증 범위는 [Current](IMPLEMENTATION_STATUS.md)에 있다.
