# promptGen 4.0.0

이미지·화면 요구를 검증해 **명확한 프롬프트·진단·실행 계약**으로 만드는 Rust 컴파일러다. CLI, 로컬 이미지 Studio, MCP와 선택 이미지 실행 adapter를 제공한다.

```bash
cargo build --release -p promptgen-cli --locked --offline
./target/release/promptgen serve
./target/release/promptgen image --input examples/image-app-icon.json --mode prompt-only --format json
./target/release/promptgen screen --input examples/screens/styled-populated.json --format json
./target/release/promptgen mcp
```

Rust 1.97.1과 lockfile의 dependency cache가 필요하다. 처음 빌드할 때는 `cargo fetch --locked`로 cache를 준비한다. [CLI 계약](crates/promptgen-cli/README.md), [MCP 연결](docs/MCP.md), [화면 가이드](docs/SCREEN_PROMPT_PLAYBOOK.md)를 참고한다.

## 이미지 작업

Studio에서 요청을 쓰면 필요한 결정만 확인하고 실제 프롬프트를 바로 보여준다. `/travel-journal /scrapbook` 등 [23개 시각 어휘](docs/IMAGE_PROMPT_RESEARCH.md)는 명시적인 시각 조건으로 풀어 쓴다. 여섯 결과물 유형과 세부 프로필을 사용하며 `travel_journal`은 사진·종이·제공 문구·사실 보존을 구분한다.

실행 backend는 `codex-subscription` 하나다. `detail=high`는 정밀한 재질·윤곽·문구를 요청하는 프롬프트 의도이며 모델의 품질 설정이 아니다. 구독 도구에 없는 모델·품질 제어를 제공한다고 주장하지 않는다. 납품 크기는 실제 PNG에서 검증하며 원본보다 큰 확대는 하지 않는다. [입력·수용 규격](docs/SPEC.md)을 따른다.

- `prompt-only`: 결정적 컴파일. 외부 모델 호출 없음.
- `luna-refine`: Codex CLI의 Luna 검토. 이미지 생성 없음.
- `codex-imagegen`: Luna 검토 후 구독으로 생성 또는 단일 base PNG 편집. 편집은 `--reference-image FILE`이 필수다.

**인증은 로그인된 Codex CLI의 ChatGPT 구독만 사용한다. API key를 요청하거나 사용하지 않는다.**

구독 편집 실행은 생성과 같은 mode에 원본 PNG를 첨부한다.

```bash
./target/release/promptgen image --input examples/image-edit-product.json --mode codex-imagegen --reference-image /path/to/base.png --image-output var/output/edited.png --result var/output/edit.receipt.json
```

원본 사진 편집은 설명·보존·변경을 분리하고 실행 시 실제 파일을 전달해야 한다. Studio는 참조 전송을 제공하지 않아 해당 편집을 이미지 생성으로 실행하지 않는다. app icon과 logo는 PNG 콘셉트이며 벡터·플랫폼 패키지를 보장하지 않는다. `screen`은 독립 strict 계약으로 CLI/Rust에서만 사용한다.

## 검증·저장

```bash
./scripts/verify.sh
./scripts/verify-quality.sh
```

[Current](docs/IMPLEMENTATION_STATUS.md)와 [검증 receipt](VERIFICATION.json)에서 실제 실행 범위를 확인한다. 과거 기록·fake provider tests는 현재 이미지 생성의 증거가 아니다. source hash가 다르면 저장된 전체 gate 성공을 현재 성공으로 사용하지 않는다.

덮어쓰기는 명시 `--force`가 필요하다. 이미지 저장·receipt·remote request는 하나의 원자 transaction이 아니다. [저장·정리 계약](docs/RUNTIME_STORAGE.md), [보안](SECURITY.md), [LICENSE](LICENSE), [NOTICE](NOTICE.md)를 보존한다. browser 이전 기록을 자동 삭제하거나 호환 변환하지 않는다.

[정체성](docs/IDENTITY_AND_EVOLUTION.md) · [아키텍처](docs/ARCHITECTURE.md) · [구현 지도](ANALYSIS.md) · [남은 검증](docs/PLAN.md)
