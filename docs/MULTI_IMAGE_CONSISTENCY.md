# 교차 이미지 연속성 계약

## 목적

PromptGen은 [ATH-MaaS/Awesome-Multi-Image-Generation](https://github.com/ATH-MaaS/Awesome-Multi-Image-Generation)의
리서치 분류를 실행 가능한 입력 계약으로 융합한다. 이 저장소는 모델·가중치·외부 API를
복사하지 않는다. 원문이 구분한 네 가지 문제를 `ImageConsistencySpec`의 네 축으로
고정하고, 같은 요청을 검증·프롬프트·생성계획·QA에서 재사용한다.

| 리서치 축 | PromptGen 값 | 검수할 불변조건 |
| --- | --- | --- |
| multi-view consistency | `multi_view` | 시점이 달라도 랜드마크, 비율, 세계 좌표, 카메라 관계 |
| character consistency | `character` | 얼굴, 머리, 의상, 실루엣, 구별 특징, 팔레트 역할 |
| temporal consistency | `temporal` | 순서, 행동·감정·소품 상태의 인과성, 이전 결과 조건화 |
| semantic consistency | `semantic` | 객체 정체성, 공간 관계, 레이아웃 위계, 행동 의미 |

## 입력 계약

`ImagePromptRequest.consistency`는 선택적 최상위 객체다. 기본 단일 이미지
요청은 기존 입력과 동일하고, 이 객체를 선언한 요청만 교차 이미지 요구와 검수 기준을 활성화한다.

```json
{
  "consistency": {
    "dimensions": ["multi_view", "character", "temporal", "semantic"],
    "shared_anchors": [
      "same heroine face and hair",
      "same wardrobe and palette roles",
      "same left-to-right screen axis"
    ],
    "sequence": {
      "steps": ["arrival-space", "letter-realization", "decision-gaze"],
      "condition_on_previous": true
    }
  }
}
```

- `dimensions`는 중복 없는 1~4개 축이다.
- `shared_anchors`는 모든 결과에서 눈으로 확인할 수 있는 1~8개 불변조건이다.
- `sequence`는 순서가 있는 결과에만 사용한다. `temporal`을 선언하면 반드시 2~24개
  단계와 `condition_on_previous=true`를 함께 선언한다.
- `cinematic_storyboard`에서는 `character`, `temporal`, `semantic`이 필수다.
  `sequence.steps`는 `storyboard.panels[].id`와 같은 순서여야 하고, `storyboard`
  정체성 앵커는 모두 `shared_anchors`에도 있어야 한다. 두 필드는 서로 다른
  책임(프로필의 시각 앵커와 교차 이미지 불변조건)을 가지지만 누락·불일치는
  검증 단계에서 차단한다.

## 렌더링과 QA

컴파일러는 모델명이나 연구 방법명을 프롬프트에 주입하지 않는다. 대신
`MULTI-IMAGE CONSISTENCY CONTRACT`에 축별 관찰 규칙을 렌더하고, generation plan
metadata에 다음을 남긴다.

- 적용 축과 공유 앵커 수
- 순서 단계 수와 이전 결과 조건화 여부
- 축별 QA 기준( `consistency_multi_view`, `consistency_character`,
  `consistency_temporal`, `consistency_semantic` )

수정은 가장 먼저 실패한 축 하나만 대상으로 한다. 이미 통과한 축, 앵커, 패널
순서, 출력 파라미터는 그대로 둔다. 이는 multi-image 연구의 서로 다른 일관성
문제를 하나의 “비슷해 보임” 점수로 뭉개지 않기 위한 경계다.

## 실행 경계

현재 PromptGen의 Codex 이미지 어댑터는 참조 이미지 transport가 검증되지 않았고,
한 번에 하나의 PNG만 생성한다. 따라서:

- `prompt-only` 컴파일은 네 축 계약과 스토리보드 시트에 사용할 수 있다.
- 참조 이미지나 이전 출력의 실제 바인딩이 필요한 실행은 검증된 어댑터가 없으면
  fail closed 한다.
- 이 문서의 `condition_on_previous`는 요구사항이지 성공을 가장하는 fallback이
  아니다. 향후 어댑터는 인덱스·receipt·재시도 경계를 추가로 증명해야 한다.

## 검증 명령

```bash
cargo test -p promptgen-core --locked --offline
cargo test -p promptgen-cli --locked --offline
python3 scripts/audit-prompts.py --check
./target/release/promptgen image \
  --input examples/image-cinematic-storyboard.json \
  --mode prompt-only \
  --format text
```

## 리서치 사용 원칙

원문 저장소는 방법, 데이터셋, 벤치마크, 응용 사례를 모은 큐레이션 인덱스로
사용한다. PromptGen이 직접 채택한 것은 네 축의 문제 분해와 그에 맞는 검수
경계뿐이다. 특정 논문·서비스의 성능이나 API를 제품 보장으로 승격하지 않으며,
모델·가중치·이미지·코드를 저장소에 복제하지 않는다.
