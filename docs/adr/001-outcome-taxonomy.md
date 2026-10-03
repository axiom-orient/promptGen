# ADR 001 · 결과물 여섯 축과 typed style 분리

## 상태

**기존 채택 결정의 재구성.** 새 category 승인이나 구현 변경이 아니다.
기존 연구 기록일은 2026-07-30, 최초 채택일은 UNKNOWN, 이번 문서 통합 확인일은 2026-09-13이다.
현재 구현 근거는 [catalog](../../catalog/image_catalog.json), [validation](../../crates/promptgen-core/src/image/validate.rs), [interview](../../crates/promptgen-core/src/interview/image.rs)다.

## 맥락

prompt collection은 탐색과 영감에 유용하지만 이 제품의 목적은 하나의 결과 의도를 typed 계약으로 만드는 것이다.
스타일·매체·결과물을 같은 category 수준에 두면 서로 다른 분류 축이 섞이고 사용자가 결과보다 구현 용어를 먼저 선택하게 된다.

## 결정

C1/C4/C5/C6/C10/C11의 여섯 primary outcome을 유지하고 medium·scene·composition·lighting·palette·surface·text·reference를 typed 필드로 분리한다.
profile은 category-bound 의미나 workflow를 표현하며 새 outcome과 같지 않다.
확장에는 기존 축으로 표현 불가능한 반복 목적과 실제 brief·대표 prompt/PNG·수용 기준·실패·비모호한 route·모든 관련 surface 일관성 근거가 필요하다.
세부 조건은 [IDENTITY](../IDENTITY_AND_EVOLUTION.md#발전-방향)가 소유한다.

## 결과

표현 범위를 단순히 줄이는 대신 결정 축을 분리한다. 기존 pattern/style ID의 묵시 변환은 하지 않는다.
세부 profile·reference·provider capability는 유지하되 gallery 크기를 제품 완성도로 평가하지 않는다.
Codex의 timeout·rate limit·fidelity 실패는 별도 실행 문제이며 category 결정이나 hash 존재로 해결되지 않는다.

## 기존 조사 출처의 보존 범위

이 목록은 통합한 과거 조사 문서의 참고문헌이다. **이번에 모두 재확인한 최신 사실이나 현재 실행 증거가 아니다.**
[OpenAI image prompting guide](https://developers.openai.com/cookbook/examples/multimodal/image-gen-models-prompting-guide),
[image eval cookbook](https://developers.openai.com/cookbook/examples/multimodal/image_evals),
[Codex imagegen skill](https://github.com/openai/codex/blob/main/codex-rs/skills/src/assets/samples/imagegen/SKILL.md),
[gallery A](https://github.com/erickkkyt/awesome-gptimage2-prompts),
[gallery B](https://github.com/YouMind-OpenLab/awesome-gpt-image-2),
[Codex timeout issue](https://github.com/openai/codex/issues/29645),
[Codex rate-limit issue](https://github.com/openai/codex/issues/26731).
현재 실행 판단에는 소스·관찰 결과를 우선하며 과거 문구의 “one focused repair”를 현재 retry 수로 복사하지 않는다.

## 2026-09-30 확장

3.0은 여섯 outcome을 유지하며 C5 안에 `travel_journal` 프로필을 추가한다. 사진·종이·제공 문구와 사실 보존은 이 프로필이 소유한다. 일반 광고의 CTA·브랜드 지시를 여행 기록에 강제하지 않는다. 현재 모델과 실행 설정은 [SPEC](../SPEC.md)와 실제 core가 소유하며 이 ADR의 이전 사례를 현재 provider 성공 증거로 쓰지 않는다.
