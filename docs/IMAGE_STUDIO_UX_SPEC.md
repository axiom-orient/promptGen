# 이미지 Studio 작업 계약

## 기본 작업

하나의 이미지 작업 공간에서 요청을 쓰고, 부족한 결정만 수정하며, 실제 프롬프트를 읽고 복사한다. 모델 실행은 별도 명시 선택이다. desktop은 최근 기록 / 입력·결정 / 결과의 세 영역이며 좁은 화면은 같은 순서로 이어진다.

- 기본 작업 크기 1440×900에서 입력과 결과를 동시에 볼 수 있어야 한다. 상단 소개·중복 탭·단계 안내가 작업물을 밀어내지 않는다.
- 생성된 프롬프트의 첫 문단은 결과 영역 상단에서 바로 읽힌다. 전체 프롬프트를 접힌 상세 안에 두지 않는다.
- single image kind에 tab switch나 cross-kind draft owner를 두지 않는다.
- 카테고리는 결과 예시 카드에서 한 번 선택한다. 검색·전체 이미지 preview·세부 프로필·키보드 이동을 유지하며 동일 범주의 filter와 density switch는 제거한다.
- `/travel-journal` 등 어휘는 서버의 동일 사전에서 읽고 입력 prefix만 바꾼다. JavaScript가 domain 표현을 따로 재정의하지 않는다.

## 필요한 상태

| 상태 | 보이는 작업과 복구 |
|---|---|
| 빈 요청 | 입력, 문서 기반 예시 두 개, 실제 대표 결과물 |
| 질문 필요 | 입력을 유지하고 부족한 결정만 inline으로 확인 |
| ready | 실제 프롬프트, 복사·요청 JSON·검증 상세 |
| 원본 사진 편집 | 원본 설명·보존·변경을 구분. CLI에 실제 base 파일을 전달해야 함을 표시 |
| 실행 중 | 실제 단계 문구와 busy 상태. 가짜 진행률 없음 |
| invalid / 실패 | 이유를 알리고 입력·수정 경로 유지. 실패를 성공 이미지로 표시하지 않음 |
| 검색 결과 없음 | 검색을 지우는 직접 복구 경로 |
| 저장 결과 복원 | 현재 실행 요청 identity를 무효화한 후 저장된 canonical 결과 적용 |

## 상태·접근성·증거

caller의 explicit answers와 서버 normalized answers를 분리한다. brief 변경·초기화·예시 취소·최근 작업 복원은 pending request identity를 갱신한다. 늦은 응답은 새 의도에 적용하지 않는다.

사진 입력이 없는 Studio는 참조가 필요한 요청을 Codex 생성으로 실행하지 않는다. 입력 파일을 실제 전달하는 Codex 구독 편집 CLI가 실행 owner다. API도 이 제한을 Luna 호출 전에 검사한다.

검증 상세와 preview는 keyboard로 열고 닫으며 focus를 복원한다. 상세 overlay를 열면 배경은 inert다. 알림은 한 polite live region으로 보낸다. 밝게·어둡게·system, reduced motion, narrow layout을 보존한다.

image publication receipt와 SHA query가 연결된 URL만 표시하며 byte mismatch는 서버가 거절한다. 표시 실패를 성공으로 유지하지 않는다. 복사하는 프롬프트는 화면에 보이는 실제 문자열이다.

local history는 `promptgen.recent.v5`다. 이전 namespace를 읽는 호환 코드는 없으며 기존 browser data를 삭제하지 않는다. 실제 조작 검증과 미확인 범위는 [Current](IMPLEMENTATION_STATUS.md)에만 기록한다.
