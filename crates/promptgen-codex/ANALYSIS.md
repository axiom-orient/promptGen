# promptgen-codex 분석

## 책임 경계

promptgen-codex는 Luna prompt review와 구독 인증으로 명시적인 이미지 생성·단일 base 편집을 수행한다. provider process, candidate, visual fidelity, PNG publication과 실행 receipt를 소유한다. 입력 의미와 canonical compile은 core의 권위다.

## Luna review와 강제 연결

1. compile된 canonical prompt와 typed request의 hash를 확인한다.
2. 설치된 Codex CLI를 명시 모델 `gpt-5.6-luna`로 호출한다.
3. 구조화된 review를 strict decode하고 policy로 승인/거절한다.
4. 승인된 source hash와 제한된 additions를 담은 `PromptRefinement`만 만든다.
5. Codex imagegen 진입은 `PromptRefinement`를 필수로 받고 refined prompt를 실행한다.

따라서 approval flag나 provider fallback으로 review를 건너뛸 수 없다. `luna-refine`는 1–4까지만 실행하며 image effect를 내지 않는다.

## Imagegen → receipt

typed request 재컴파일 전체가 caller의 compilation과 일치해야 process가 시작된다. event/thread/call을 admission한 뒤 candidate를 private workspace에 복사하고 그 snapshot에서 PNG와 fidelity를 검사한다. repair가 발생하면 final turn의 metadata/evidence가 receipt에 귀속된다. validated SHA와 publication copy SHA가 같아야 게시한다.

timeout/nonzero는 remote 결과 미실행을 뜻하지 않는다. local output이 게시된 뒤 durability나 receipt 단계에서 실패하면 경로와 SHA를 보고한다. 원격 취소나 idempotency는 제공하지 않는다.

## 검증 상태

실제 `gpt-5.6-luna` prompt review는 동작했다. gate의 adapter tests는 process/event/fidelity/publication 경계를 검증하지만 real imagegen end-to-end 증거는 아니다. imagegen, visual grader 정확도, provider remote cancellation의 현재 확인 범위는 [구현 현황](../../docs/IMPLEMENTATION_STATUS.md)에 있다.


## 구독·원본 경계

API key는 요청하거나 사용하지 않는다. 모든 provider process가 `forced_login_method="chatgpt"`와 Luna model 설정을 명시하고 API-key 환경 변수를 제거한다. 편집에서는 원본을 private PNG snapshot으로 고정하고 SHA를 receipt에 기록한다. 검수에는 원본과 후보를 구분해 첨부한다. receipt는 `codex-subscription` 경로와 prompt의 detail 의도를 기록하되, tool이 노출하지 않는 이미지 모델은 `null`로 남긴다.
