# promptGen 문서 색인

전체 사실은 Current, 확정된 의미는 규범, 남은 결정·구현·검증은 PLAN에서 관리한다. 상세 분석을 상위 문서에 다시 복제하지 않는다.

| 정본 | 책임 |
|---|---|
| [IMPLEMENTATION_STATUS](IMPLEMENTATION_STATUS.md) | 제품·6-crate graph·기능 요약·검증·UNKNOWN |
| [IDENTITY_AND_EVOLUTION](IDENTITY_AND_EVOLUTION.md) | 정체성·불변조건·허용 변화·발전 원칙 |
| [SPEC](SPEC.md) | 필수/선택 기능과 관찰 가능한 수용 기준 |
| [ARCHITECTURE](ARCHITECTURE.md) | 확정된 책임·의존 방향·실행·상태·효과 경계 |
| [PLAN](PLAN.md) | 남은 결정·구현·검증과 완료 조건 |
| [루트 ANALYSIS](../ANALYSIS.md) | 통합·catalog·운영 도구·증거와 material-unit 판정 |
| [crate 분석 색인](../crates/README.md) | core·CLI·Codex·Web·MCP의 상세 경계 |

## 독립 독자와 상세 계약

| 문서 | 독자·목적 |
|---|---|
| [IMAGE_PROMPT_SPEC](IMAGE_PROMPT_SPEC.md) | typed 이미지 요청 작성자·provider 구현자의 task/profile/reference 계약 |
| [POSE_TRANSFER_PROMPT_SPEC](POSE_TRANSFER_PROMPT_SPEC.md) | 포즈/외형 참조 권위와 검수 순서 |
| [CINEMATIC_STORYBOARD_PROFILE](CINEMATIC_STORYBOARD_PROFILE.md) | C10 스토리보드 입력·패널 계약 |
| [MULTI_IMAGE_CONSISTENCY](MULTI_IMAGE_CONSISTENCY.md) | 교차 이미지의 요구·관찰 기준 |
| [SCREEN_PROMPT_PLAYBOOK](SCREEN_PROMPT_PLAYBOOK.md) | 화면 입력·좌표·상태·13개 예제 사용 |
| [IMAGE_STUDIO_UX_SPEC](IMAGE_STUDIO_UX_SPEC.md) | Studio의 사용자 결정·결과 귀속 |
| [DESIGN](DESIGN.md) | 소스·테스트가 참조하는 시각·접근성 조항 |
| [IMAGE_INTERVIEW_SCENARIOS](IMAGE_INTERVIEW_SCENARIOS.md) | 대표 route와 상태 회귀 수용 기준 |
| [MCP](MCP.md) | MCP client·운영자의 연결·metadata·도구·오류 |
| [RUNTIME_STORAGE](RUNTIME_STORAGE.md) | 사용자 입력·결과·임시 파일·삭제 운영 |
| [SECURITY](../SECURITY.md) | 신뢰 경계·방어·알려진 한계·문제 보고 |
| [catalog](../catalog/README.md) | catalog 원본·생성 경로·PNG 증거 |

## 결정과 생성·검증 자료

[ADR 001](adr/001-outcome-taxonomy.md)은 기존 여섯 outcome/typed style 분리 결정,
[ADR 002](adr/002-screen-contract.md)는 기존 screen의 원문·좌표·상태 전달 결정을 보존한다.
새로운 외부 host 통합이나 provider 확장을 승인하는 문서가 아니다.

[PROMPT_AUDIT_REPORT](PROMPT_AUDIT_REPORT.md)는 생성기 산출물이며 직접 편집하지 않는다.
[이미지 생성 payload](../catalog/IMAGE_GENERATION_PROMPTS.md)와 날짜가 붙은 `docs/research/` 자료는 참고 자료다. 검증을 위해 provider 호출을 반복하는 dogfood 작업은 제품 구현 완료의 선행 조건이 아니다.

- [최신 이미지 프롬프트 리뉴얼 근거](IMAGE_PROMPT_RESEARCH.md): 2026-09-30 문서 기준 조사·어휘·보존·검수 계약.
