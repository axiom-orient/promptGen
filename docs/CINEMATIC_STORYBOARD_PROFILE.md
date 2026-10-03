# Cinematic storyboard profile

`render_profile=cinematic_storyboard`는 C10(일러스트·스토리) 안에서만 쓰는 텍스트 없는 3·4·6패널 시각 전달물 계약이다. 기획자·디자이너가 화면 자체만 보고 인물 연속성, 감정 비트, 시선선, 화면 방향, 샷 구도를 검토하는 데 사용한다. 설명문·패널 번호·대사·말풍선은 이미지 안에 넣지 않는다.

이 프로필은 [ATH-MaaS/Awesome-Multi-Image-Generation](https://github.com/ATH-MaaS/Awesome-Multi-Image-Generation)의
`multi_view`, `character`, `temporal`, `semantic` 문제 분해를 `consistency`
필드로 함께 선언한다. `character`, `temporal`, `semantic`은 필수이고,
멀티뷰 샷 래더를 실제로 사용할 때 `multi_view`를 추가한다.

새 최상위 taxonomy 카드를 만들지 않는다. C10은 이미 스토리보드를 결과물로 소유한다. 차이는 매체나 분위기가 아니라 **여러 샷 사이의 연속성 계약**이므로, `cinematic_storyboard`는 C10의 실행 프로필이다. 이 결정은 카드 수를 늘리지 않으면서 새 검증 경계를 만든다.

## 입력 계약

`storyboard`는 이 프로필에서 필수다.

| 필드 | 역할 |
| --- | --- |
| `layout` | `three_panel_strip`, `two_by_two`, `three_by_two` 중 하나. 레이아웃이 패널 수를 고정한다. |
| `screen_direction` | 전체 시퀀스의 `left_to_right` 또는 `right_to_left` 축. |
| `identity_anchors` | 3~6개의 눈에 보이는 불변점. 머리·의상·소품·실루엣·팔레트 역할처럼 패널마다 확인할 수 있어야 한다. |
| `panels[]` | 순서가 있는 샷 목록. 각 패널은 `shot_size`, `camera_angle`, `camera_move`, `action`, `body_facing`, `gaze_target`, `emotional_beat`을 가진다. |
| `consistency` | 네 일관성 축, 공유 앵커, 패널 ID와 일치하는 temporal sequence를 가진다. |

공통 `subjects[]`는 인물의 정체성을, `storyboard.panels[]`는 패널별 행동과 카메라를 소유한다. 같은 책임을 두 곳에 중복하지 않는다.

## 프롬프트 설계 원칙

1. 먼저 **정체성 앵커**를 잠근다. 이름이나 추상적 성격이 아니라 얼굴 비율, 헤어, 의상, 소품, 색 역할처럼 보여서 비교할 수 있는 값만 쓴다.
2. 패널 하나에는 **사건·감정 전환 하나**만 둔다. 한 패널 안에 발견·회상·결심을 모두 넣지 않는다.
3. 샷은 공간을 세우고, 감정을 좁히고, 시선의 목적지를 남기는 순서로 쓴다. 예: establishing → medium close-up → close-up.
4. `screen_direction`을 뒤집지 않는다. 뒤집어야 하면 중립적인 재설정 샷을 별도 패널로 선언한다.
5. 앞 패널의 `gaze_target`과 다음 패널의 대상·공간을 연결한다. “옆을 본다”가 아니라 `offscreen_right`, `object_in_frame`처럼 명시한다.
6. 생성 이미지 안에는 글자를 넣지 않는다. 코드·HTML·PDF의 외부 메타데이터가 장면 ID, 설명, 승인 상태를 소유한다.
7. 멀티뷰·캐릭터·시간·의미를 한 문장의 “일관성”으로 합치지 않는다. 각 축을
   별도 QA 대상으로 보고 가장 먼저 실패한 축 하나만 교정한다.

## 민아 예제

[`examples/image-cinematic-storyboard.json`](../examples/image-cinematic-storyboard.json)은 GeultuStudio의 “시각” 단계에 맞춘 세 패널 예제다.

- 1: 승강장 설정 샷, 민아가 화면 오른쪽 열차 방향을 본다.
- 2: 편지를 확인하는 미디엄 클로즈업, 시선이 프레임 안 편지로 이동한다.
- 3: 같은 축의 얼굴 클로즈업, 오른쪽 오프스크린 목적지로 시선을 되돌린다.

```bash
cargo run -p promptgen-cli --locked --offline -- \
  image --input examples/image-cinematic-storyboard.json --mode prompt-only --format text
```

## 검수와 수정

다음 순서로만 판정한다. 앞 단계가 실패하면 뒤 단계의 예쁨은 통과 근거가 아니다.

1. `identity_anchors`: 같은 인물·의상·소품·색 역할인가.
2. `beat_progression`: 각 패널이 선언한 행동과 감정 전환 하나만 보이는가.
3. `screen_direction_eyeline`: 축을 넘지 않고 앞 패널의 시선과 다음 패널의 공간이 이어지는가.
4. `textless_handoff`: 패널 번호·캡션·말풍선·대사·로고·워터마크·UI·의사문자가 없는가.

수정은 가장 먼저 실패한 항목 하나만 바꾼다. 예를 들어 2번 패널의 눈이 왼쪽으로 향하면 `panel_2.gaze_target`과 그 패널의 카메라 각도만 고치고, 승인된 정체성 앵커·샷 크기·화면 방향은 보존한다.

## 리서치 근거

- OpenAI는 이미지 요청을 목적·주제·행동·장소·스타일로 구체화하고, 참조 수는 적게 관리하는 방식을 권장한다. 이 프로필은 이를 패널별 카메라·행동·정체성 필드로 분해한다. [OpenAI Academy: Creating images with ChatGPT](https://openai.com/academy/image-generation/)
- Adobe의 문서는 프롬프트에서 카메라 각도·구도·조명·색을 명시해 결과를 제어하는 방식을 제시한다. 여기서는 이를 “샷 크기·각도·이동”으로 분리했다. [Adobe Firefly prompting guidance](https://helpx.adobe.com/firefly/web/work-with-audio-and-video/work-with-video/writing-effective-text-prompts-for-video-generation.html)
- 스토리보드 연속성 자료는 같은 축에서 인물의 화면 좌우와 카메라 위치를 유지해야 편집 시 공간 혼란을 줄인다고 설명한다. 그래서 `screen_direction`과 `gaze_target`을 필수로 만들었다. [StoryboardArt: continuity](https://storyboardart.org/storyboard-tutorials/continuity-for-storyboards/)
- 다중 이미지 생성 리소스는 정체성, 시점, 시간, 의미 연속성을 서로 다른 문제로 본다. 따라서 “같은 캐릭터” 한 문장 대신 네 개의 QA 게이트로 분리했다. [Awesome Multi-Image Generation](https://github.com/ATH-MaaS/Awesome-Multi-Image-Generation), [CharaConsist](https://github.com/Murray-Wang/CharaConsist)

이 링크들은 프롬프트·촬영 문법의 근거이며, 런타임 의존성이나 외부 API 계약은 아니다.
