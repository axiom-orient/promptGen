# 남은 구현 및 출시 결정

제품 상태와 마지막으로 수행한 검증의 시점은 [구현 현황](IMPLEMENTATION_STATUS.md)에서 관리한다. root `VERIFICATION.json`은 마지막 성공 gate의 source identity이며, 뒤이은 문서 수정에는 재검증이 수행되지 않았다.

## 현재 구현 결과

CLI, loopback Studio, MCP와 typed prompt compiler가 단일 `codex-subscription` 실행 경로에 연결되어 있다. 이미지 실행에는 로그인된 Codex CLI와 Luna prompt review가 필요하다. 구독으로 실제 생성·base 편집을 수행한 기록이 있으며, API key 경로는 없다.

현재 이미지 모델은 정확한 포스터 타이포그래피 사례에서 네 번 교정 후에도 요구한 절개선 수를 맞추지 못했다. 실행기는 실패 후보를 게시하지 않고 오류를 반환한다. 이는 hidden success fallback이 없다는 증거이지만, 해당 사례의 품질 합격 증거는 아니다.

추가 provider dogfood를 반복하는 것은 구현 완료의 조건이 아니다. 기록된 실행 결과와 자동 검수 경계를 현재 근거로 사용하고, 추가 구독 실행은 제품 변경이나 사용자의 구체적인 요청이 있을 때 진행한다.

## 대외 출시를 선택할 경우

- 정확한 문구·기하 효과가 항상 맞아야 한다면 이미지 모델의 프롬프트 재현에 의존하지 않는 deterministic typography/rendering 경계를 별도로 설계하고 구현한다.
- Studio의 브라우저 키보드 흐름과 VoiceOver 수용은 출시 플랫폼 범위를 정한 뒤 평가한다. 현재 UI 계약 테스트는 사람 평가를 대체하지 않는다.
- public hosting, 다중 참조, batch, 원격 취소, 중복 방지, 내구성 있는 작업 이력은 현재 로컬 제품 계약에 포함되지 않는다.

이 항목들은 현재 사용을 위한 추가 dogfood 요구가 아니라 대외 출시 시 결정할 제품·운영 범위다.
