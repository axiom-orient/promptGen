# promptGen 4.0.0 구현 현황 — 2026-10-01

이 문서는 현재 코드와 검증 증거의 상태를 기록한다. 제품 계약은 [SPEC](SPEC.md), local gate의 source identity는 root [VERIFICATION.json](../VERIFICATION.json)이 소유한다. source revision은 [REVISION](../REVISION)이다. 저장소에 `.git`이 없어 commit·branch·diff 정보는 `[UNKNOWN]`이다.

## 구현 및 실제 연결

- core는 strict typed request를 검증해 deterministic prompt와 진단을 만든다. 기본 동작은 provider-free다. 이미지 실행은 명시적으로 요청한 CLI/Studio 명령만 시작한다.
- 이미지 실행 전 Codex CLI의 `gpt-5.6-luna`가 canonical prompt를 검토한다. 원문 prompt/hash에 묶인 제한된 additions만 허용하며 Luna가 거절하거나 잘못된 구조를 반환하면 이미지 실행을 시작하지 않는다.
- 하나의 이미지 실행 경로는 로그인된 ChatGPT 구독 인증을 강제한다. API key는 요청·읽기·설정·사용하지 않는다. Codex 자식 process에서도 API-key 환경 변수를 제거한다.
- 실제 이미지 tool은 model·size·quality 인자를 제공하지 않는다. `backend=codex-subscription` 및 `detail`은 납품 대상과 prompt에 넣는 시각 의도다. 실행 receipt의 관찰 이미지 모델은 확인할 수 없으면 `null`로 남긴다. 요청 설정을 provider가 적용한 사실처럼 기록하지 않는다.
- 편집은 bounded PNG 한 장을 private snapshot으로 고정하고 SHA를 기록해 실제 이미지 편집 입력으로 전달한다. 원본·출력 alias, symlink, hardlink를 거절하고 기존 결과는 명시 overwrite 없이는 바꾸지 않는다.
- 생성 후보는 thread/event, PNG·크기·비율, 요청의 문구·개수·배치, receipt SHA를 검사한 뒤 게시한다. `separator_count`가 선언된 문구는 원하는 숫자를 보여주지 않은 Luna 관찰값을 로컬 코드가 대조한다. 전체 model pass라도 숫자가 다르면 거절한다.
- 후보의 개수·배치·분리선·정확 문구가 검수에서 통과한 경우, 다음 한 축 교정 입력에 해당 통과 계약을 구조화해 전달해 수정 중 regression을 막는다. 검수 결과가 없거나 실패한 조건은 보호 조건으로 승격하지 않는다.
- Studio·CLI·MCP는 동일 core 계약을 사용한다. Studio/MCP HTTP는 loopback·정적 asset·요청 제한을 적용하며, MCP 원격 연결은 별도 bearer/Host 경계를 요구한다.

## 확인된 결과

| 검사 | 결과 |
|---|---|
| 마지막 `./scripts/verify.sh` 성공 | revision `promptgen-4.0.0-20261001.2`에서 299개 PASS, live qualification 1개 ignored. check/test/doc/build 및 정적 감사를 통과했다. root `VERIFICATION.json`은 이 문서 갱신 전 source tree를 가리킨다. |
| 이후 게이트 시도 | 실패. 문서 링크 검사에서 존재하지 않는 `../evals/dogfood/current/IMAGE_EVIDENCE_POLICY.md`를 지적했다. 이 문서에서 해당 링크를 제거했다. 사용자의 요청에 따라 재검증하지 않았으므로 수정 결과는 `[NOT_RUN]`이다. |
| 마지막 `./scripts/verify-quality.sh` | 문서 갱신 전 revision `.2`에서 fmt·Clippy `-D warnings` PASS. 이후 실행하지 않았다. |
| 실제 생성 및 편집 | ChatGPT 구독으로 PNG 생성과 base 사진 편집을 실행했다. 편집 결과는 지정된 머그컵 색만 바뀌고 인물·구도·배경이 보존됐다는 실제 fidelity receipt를 남겼다. 결과와 receipt는 ignored local evidence인 `var/qualification/20261001/portrait-mug-subscription.png` 및 `.receipt.json`에 있다. |
| 정확 문구 포스터 end-to-end | 최신 `.2` 릴리스로 실제 구독 실행을 다시 했지만 4회 뒤 분리선 mismatch `[2, 2]`로 종료했다. PNG와 성공 receipt는 게시되지 않았다. 이미 통과한 계약을 보정 입력에 담도록 고쳤고 회귀 테스트는 통과했지만, 이 사례의 이미지 품질은 여전히 불안정하다. raw 결과는 `var/qualification/20261001/poster-live-v4-repair-preservation.run.log`에 있다. |
| 실제 separator 실패 검수 | 실제 포스터 PNG를 최신 Luna 분리선 관찰에 전달했다. 각 줄에서 1개가 보여 expected 3과 다르다는 결과를 받고 local gate가 `pass=false`로 반환했다. 실제 관찰 receipt는 이 turn의 tool output 및 `var/qualification/20261001/rejected-separator-live-regression.log`에 있다. |
| release 실행 | `target/release/promptgen serve --bind 127.0.0.1:4187 --output-dir var/output` 실행 중. health가 `4.0.0`을 반환했다. [Studio](http://127.0.0.1:4187) |
| browser·사람 검증 | UI 계약 테스트와 release HTTP smoke는 PASS. 이번 세션의 Aside daemon은 시작되지 않아 실제 브라우저 상호작용·좁은 창·키보드 탐색·VoiceOver와 사용자 평가는 `[UNVERIFIED]`다. |

## 출시 판단과 남은 확인

구독 생성·편집 경로는 실제로 연결되어 있다. 정확 문구 포스터는 4회 교정 뒤에도 분리선 수가 틀려 거절됐고 게시하지 않았다. 이 사례에서 이미지 결과 품질은 합격하지 못했다. 브라우저·VoiceOver·사용자 평가는 수행하지 않았다. 구독 tool이 실제 이미지 모델을 노출하지 않아 반복 품질을 보장하지 않으며 원격 취소·멱등성도 제공하지 않는다.

추가 provider dogfood는 현재 작업의 완료 조건이 아니다. 제품을 대외 출시하기로 할 때 브라우저·VoiceOver 수용 기준을 별도로 정하고, 정확 타이포그래피가 필수라면 이미지 모델의 우연한 재현에 맡기지 않는 deterministic rendering 구현이 필요하다. 범위는 [남은 작업](PLAN.md), 오류·신뢰 경계는 [SECURITY](../SECURITY.md)에 기록한다.

## 보존된 자료와 단일 계약

활성 실행은 `codex-subscription` 하나다. API 모델·품질 제어, 직접 API adapter, migration/fallback 경로는 없다. Catalog PNG·fixtures·legal files·날짜가 붙은 연구 자료는 해당 계약의 참고 자료로 유지하며 현재 runtime 성공 증거로 승계하지 않는다. v4는 clean-break 계약이며 이전 browser history namespace를 읽거나 변환하지 않는다.
