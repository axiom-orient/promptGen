# promptgen-codex

`execute_image_generation`을 제공하는 공개 Codex adapter다. 호출자는 이미 컴파일한 image 요청과 실행 configuration을 전달한다.
명시한 Codex executable과 `CODEX_HOME`, 선택적인 Codex executable SHA-256 pin,
read-only sandbox·event 계약, thread-scoped PNG 후보, count/fidelity 검수와 제한된 repair,
PNG 게시·receipt를 소유한다. SHA pin을 주면 version probe, Luna review, generation, fidelity와
repair child를 띄우기 직전에 identity를 다시 확인하고 execution receipt에 digest를 기록한다.

이 adapter는 generate/no-reference 범위다. 참조 경로를 prompt에 적는 것으로 파일을 전달하지 않는다.
실제 Codex tool 호출과 모델 시각 판단은 별개의 신뢰 경계이며 event 사후 검사는 이미 수행된 외부 효과를 되돌리지 않는다.
PNG와 CLI receipt는 하나의 transaction이 아니고 overwrite가 전체 rollback을 보장하지 않는다.
macOS `sips`가 필요한 정규화 조건, timeout과 실패·partial effect는 [ANALYSIS](ANALYSIS.md)에 있다.
공개 계약의 원본은 [src/lib.rs](src/lib.rs), 남은 작업은 [PLAN](../../docs/PLAN.md)이다.
