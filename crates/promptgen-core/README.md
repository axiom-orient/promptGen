# promptgen-core

image·screen 의도를 검증하고 결정적으로 프롬프트로 만드는 공개 Rust library다.
image guided interview, 여섯 outcome catalog, category-bound profile, schema, JSON parser와 PNG/SHA utility를 제공한다.

공개 surface의 원본은 [src/lib.rs](src/lib.rs)와 각 모듈 export다.
`ImagePromptRequest`, `compile_image_prompt`, `validate_image_request`, `run_interview`,
`screen::compile`, `ScreenRequest`, catalog/schema 함수가 핵심 진입점이다.
compiler 계산은 provider·계정·파일 쓰기를 시작하지 않는다. 단, PNG path convenience API에는 실제 파일 읽기가 있다.
reference 설명과 실제 이미지 전송은 별개이며 caller가 파일 전달을 책임진다.

`app_icon`/textless `app_web_ui`/pose/storyboard의 의미는 [이미지 규격](../../docs/IMAGE_PROMPT_SPEC.md),
화면 입력은 [화면 가이드](../../docs/SCREEN_PROMPT_PLAYBOOK.md)를 따른다.
현재 공개 경계·typed render 전제·검증 범위는 [ANALYSIS](ANALYSIS.md)에서 확인한다.
