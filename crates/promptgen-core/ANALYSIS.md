# promptgen-core 분석

## 책임 경계

core는 image/interview/screen 의미, strict decode, 교차 검증, 결정적 출력의 권위다. provider 인증·네트워크·프로세스·게시 상태는 소유하지 않는다. image artifact의 파일 helper는 명시적인 bounded I/O surface다.

## 공개 flow

| 기능 | Caller → authority | 입력·검증 | 결과 |
|---|---|---|---|
| Image compile | CLI/Web/MCP/provider → `compile_image_prompt` | typed request, category/profile/task/reference 계약 | `CompilationOutcome` |
| Interview | CLI/Web/MCP → `run_interview` | brief + 허용 answer key | `needs_input` / `ready` / `invalid` |
| Screen compile | CLI/Rust → `screen::compile` | strict input, geometry/state validation | hash-bound handoff |
| Catalog/schema/LUT | adapters → typed core values | static catalog v5와 typed enums | JSON/data |
| PNG/SHA | adapters/Rust → artifact APIs | strict PNG decode, bounded file read | dimensions/hash 또는 오류 |

```text
image JSON/typed → decode → validate → category/profile decision → prompt
brief + answers  → allowlist check → interview → typed image request → compile
screen input     → decode → validate → fallible render → hash-bound handoff
PNG path         → bounded read → structural decode → dimensions/hash
```

Image taxonomy는 `taxonomy.category` 하나만 받는다. 여섯 대표 결과물은 schema v5 catalog에 있고 `tier_2`는 UI 탐색 그룹이다. Interview JSON decoder와 `run_interview`는 등록되지 않은 answer key를 거부한다. Screen typed API의 `render()`는 `Result<String, String>`을 반환하고 직접 구성한 invalid state도 validation 뒤에만 렌더한다.

task별 reference 계약도 한 정본으로 유지한다. generate는 subject/style/product/layout/palette reference를 허용하고 base/mask는 거부한다. edit는 base 하나를 요구한다. composite는 두 개 이상 reference와 base 또는 layout anchor가 필요하며 base는 최대 하나다. `pose_transfer`는 pose/subject 두 reference의 권위를 분리한다.

## 검증 상태

워크스페이스 검증은 [구현 현황](../../docs/IMPLEMENTATION_STATUS.md)이 소유한다. 변경 시에는 `cargo fmt --all -- --check`, core/workspace tests, clippy/doc 검사를 수행한다. PNG read limit·provider candidate admission·prompt fidelity는 개별 경계에서 검증되며, prompt 검증은 실제 생성 이미지의 미적 품질을 증명하지 않는다.
