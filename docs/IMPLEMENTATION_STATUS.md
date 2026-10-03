# promptGen 5.0.0 구현 현황 — 2026-10-03

현재 제품 계약은 [SPEC](SPEC.md), source identity는 [REVISION](../REVISION)과
[VERIFICATION.json](../VERIFICATION.json)이 소유한다. UpAgent는 standalone source를
별도 checkout으로 pin해 빌드한다. 포함된 GitHub verification receipt는 현재
maintained source tree에 대한 `./scripts/verify.sh` 결과다.

## 구현 및 실제 연결

- `agent-contract`는 공통 compiler outcome·진단·semantic receipt와 provider 타입의
  단일 source다. 기본 feature는 std-only이며 runtime/I/O/인증을 의존하지 않는다.
  UpAgent와 Vergerail이 같은 타입을 직접 사용한다. `schema shared-contract`로 공통
  카탈로그를 출력한다. v5 compiler wire·Studio 결과 형식은 보존한다.

- core는 strict typed image/screen request를 검증하고 deterministic prompt와 진단을
  만든다. `prompt-only`는 provider-free compile이다.
- `luna-refine`과 `codex-imagegen`은 로그인된 ChatGPT 구독을 쓰는 Codex CLI만 사용한다.
  API key 경로는 없고 provider child에서 `OPENAI_API_KEY`와 `CODEX_API_KEY`를 제거한다.
- `--codex-sha256`은 절대 경로의 regular executable을 제한된 크기 안에서 stream-hash해
  고정한다. 버전 조회, review, image generation, fidelity check, repair process 직전에
  identity를 다시 확인하고 execution receipt에 digest를 남긴다.
- `--max-fidelity-attempts 1..=4`는 한 codex-imagegen 실행의 후보 상한이다. 기본값은
  4이며 다른 mode에서는 거절한다. 호출자는 외부 approval/effect 예산에 맞춰 후보 수를
  줄일 수 있다.
- `codex-imagegen`은 reference 없는 이미지 생성 또는 명시적으로 전달된 base PNG 편집을
  실행한다. source snapshot, 실제 reference 입력, 출력 SHA·PNG 검사와 fidelity receipt를
  결합한다. 원본 파일은 덮어쓰지 않는다.
- 실제 이미지 tool은 model·size·quality 인자를 제공하지 않는다. `backend`와 `detail`은
  납품 계약·프롬프트 의도이고, 실제 PNG에서 확인할 수 없는 provider 설정은 적용 사실로
  기록하지 않는다.
- CLI, local Studio, MCP는 typed core를 사용한다. UpAgent 소비자는 `prompt-only` compile과
  명시 승인된 one-candidate semantic edit만 별도 authority로 연결한다.

## 검증

```bash
./scripts/verify.sh
```

`VERIFICATION.json`은 현재 revision과 maintained-source tree hash를 묶은 local gate
receipt다. check/test/doc/clippy/build, maintained-source audits, CLI smoke와 Studio HTTP smoke를
통과해야 한다. 이 checkout에는 1.4 historical image evidence archive가 없어 해당 증거 audit는
`NOT_RUN`으로 receipt에 기록되며, 누락된 증거를 복원하거나 성공으로 바꾸지 않는다. Live
subscription effect는 이 local gate에서 실행하지 않는다.

## 보존된 runtime evidence와 한계

4.0 source에서 실제 ChatGPT 구독 이미지 편집이 수행되어 제한된 머그컵 색 변경과 인물·구도·
배경 보존을 확인한 receipt가 있었다. 정확 문구 포스터는 네 번 보정 뒤 분리선 수가 맞지 않아
게시되지 않았고 별도 fidelity 관찰에서 거절됐다. 이 결과는 4.0의 과거 evidence이며 5.0
binary나 새 host integration의 live runtime proof로 승계하지 않는다. evidence는 ignored local
`var/` 아래에 있어 source package에는 포함되지 않는다.

5.0의 변경은 실행 후보 수 상한과 Codex binary identity pin이다. 실제 subscription 생성·편집,
시각 품질, browser/keyboard/VoiceOver 사용성, remote cancellation·idempotency는 별도
runtime qualification 없이는 `[UNVERIFIED]`다. Codex tool이 모델 ID를 노출하지 않는 실행은
관찰된 사실만 receipt에 남기며 품질을 보장하지 않는다.

## 단일 실행 계약

활성 이미지 실행 backend는 `codex-subscription` 하나다. 직접 API adapter와 API-key fallback은
없다. Catalog·fixtures·legal files·dated research는 source/reference 자료이며 현재 실행 성공
증거가 아니다. 변경된 v5 public Rust configuration에 대해 workspace major version을 5로
올렸다.
