# promptgen-cli 분석

## 책임 경계

promptgen-cli는 workspace의 단일 process composition root다. input option·stdin·stdout·파일 publication·exit code를 소유하고 domain 의미는 core, 외부 provider 효과는 전용 adapter에 둔다.

## 이미지 mode

| mode | 동작 |
|---|---|
| `prompt-only` | deterministic compile 결과를 text/JSON으로 반환 |
| `luna-refine` | Codex CLI의 `gpt-5.6-luna`로 prompt를 검토·제한 보완, 이미지 실행 없음 |
| `codex-imagegen` | Luna proof가 만들어진 prompt만 Codex 이미지 실행에 전달 |
| `codex-imagegen --reference-image` | Luna proof와 base image 하나를 Codex 단일 base edit에 전달 |

실제 image 실행은 유효한 `PromptRefinement`이 없으면 provider를 호출하지 않는다. 이미지 관련 output은 `--result`와 `--image-output`로 각각 지정하며 같은 filesystem object를 가리키면 dispatch 전에 거절한다. 이전 `--output` 별칭과 `--family` option은 없다.

## 파일과 partial failure

경로 alias는 lexical path, 기존 상위 경로의 canonical identity, Unix existing-file identity를 비교한다. result write는 temp→fsync→publish→parent fsync를 사용한다. 이미지가 이미 게시된 뒤 receipt write가 실패하면 오류에 published path/SHA를 포함하고 성공/미게시로 가장하지 않는다. `--force`는 로컬 overwrite 권한만 뜻한다.

## 검증 상태

`./scripts/verify.sh`는 CLI unit/integration, release build, CLI smoke를 실행한다. 실제 구독 생성·편집과 남은 browser 검증의 현재 결과는 [구현 현황](../../docs/IMPLEMENTATION_STATUS.md)에 있다.
