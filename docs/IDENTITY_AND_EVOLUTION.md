# promptGen 정체성과 발전 방향

## 정체성

promptGen의 목적은 이미지·화면을 원하는 사람이 자신의 의도를 **검증 가능한 구조화 요청과 결정적 프롬프트**로 고정하도록 돕는 것이다. 사용자는 CLI/Studio 사용자이고 caller는 Rust host·MCP agent·외부 process 소비자다. 자연어의 모호함, 카테고리와 스타일의 혼동, 참조 역할 충돌, 입력과 결과의 출처 혼동을 줄이는 것이 핵심 가치다.

코어는 image/screen의 입력 의미·검증·진단·render를 책임진다. image interview는 필요한 결정만 질문하고 caller가 답을 누적한다. 선택 provider는 **명시적으로 요청한** 이미지 실행과 산출물 검증을 담당한다. compiler의 성공과 이미지 생성 성공은 같은 약속이 아니다.

범용 agent 운영체제, 파일/계정의 암묵적 탐색, 무제한 생성 갤러리, 무조건적 미적 품질·pixel-perfect 결과, 화면 PNG를 실제 서비스로 납품하는 일, SVG/vector·플랫폼 icon package 보장은 비목표다. 외부 host의 승인·작업 예산·원본 저장·CAS를 promptGen이 대신 소유하지 않는다.

## 변하면 안 되는 것

image와 screen의 독립된 typed 계약, 유효한 입력의 결정적 render, 명시적 diagnostics, strict decode/교차 검증, `needs_input`/`ready`/`invalid` 의미, provider 없는 prompt-only 사용을 보존한다. input/prompt hash는 bytes 결합이지 권한이나 시각 정확성의 증명으로 바꾸지 않는다.

여섯 primary outcome C1/C4/C5/C6/C10/C11의 의미와 category-bound profile을 보존한다. medium·lighting·palette·layout·text·reference를 별도 카테고리 축으로 중복시키지 않는다. `pose_transfer`/storyboard/consistency는 각자의 참조·연속성 계약이지 새로운 taxonomy card가 아니다. `app_icon`은 C4의 단일 중앙 subject·무문구·불투명 1024² PNG master concept이고 `logo_identity`는 vector 보장이 아닌 PNG concept다. textless `app_web_ui`는 발명한 문구·숫자·가짜 문자·데이터/차트 없이 구조와 상태를 표현한다.

원본/reference bytes, 사용자 결과, 공개 schema/CLI/library/MCP와 receipt의 intentional contract, 명시 overwrite/인증/실패 의미를 보존한다. 요청한 변경과 보존 범위를 구별하고 실제로 전달하지 않은 파일을 참조했다고 주장하지 않는다. 컴파일 계산은 외부 I/O에서 분리한다. 공개 파일 편의 API의 존재 자체를 이유로 사용자 계약을 무단 폐기하지 않는다. Codex runtime SHA pin이 설정되면 각 child spawn 직전에 같은 regular non-symlink 실행 파일인지 확인하고 receipt에 digest를 남긴다. LICENSE/NOTICE와 attribution을 유지한다.

## 변경 가능한 것

같은 의미·보존·권한·오류 계약 아래 UI 표현, 내부 함수/모듈 구성, provider, transport, 저장 adapter, 성능 구현을 교체할 수 있다. domain 의미가 같다고 모든 provider의 auth·reference·timeout·commit을 공용 추상화에 숨길 필요는 없다. 신뢰 경계별 반복 validation은 제거 대상인 의미 중복과 다르다.

schema 변경, prompt 정책 변경, transport 추가, public API 재배치, receipt outcome 확장은 각각 소비자 영향과 버전 의미를 확인해 결정한다. core 컴파일 기능의 순수성과 user-facing 계약을 유지하며 현실의 optional capability·복구·운영 경고를 감추지 않는다. “교체 가능”은 추가 구현이나 호환 폐기 권한을 자동 부여하지 않는다.

## 발전 방향

더 많은 기능보다 **의도 → typed contract → deterministic compile → explicit effect → observed artifact → evidence**의 동일성을 우선한다. 모델이 수행할 요구와 프로그램이 강제할 권한, 실제로 관찰한 결과를 분리한다. 이미지 품질 평가는 원본 입력과 artifact bytes에 연결하고 실패를 일반적인 성공률이나 새로운 prompt 효능으로 과장하지 않는다.

새 outcome은 기존 여섯 outcome과 typed 필드로 표현할 수 없는 반복 목적, 최소 세 개의 구별되는 실제 brief, 대표 prompt/PNG/수용 기준/실패 사례, 기존 route와의 비모호성, CLI·Studio·schema·audit의 일관된 지원 근거가 있어야 한다. 갤러리 수나 스타일 유행만으로 확장하지 않는다. [taxonomy 결정](adr/001-outcome-taxonomy.md)을 따른다.

screen은 목적·사용자·주요 행동·정보/상태·기하 제약을 더 명확히 전달하는 방향으로 발전시킨다. 작동하는 UI, 접근성, 승인/복구는 해당 결과 owner와 연결해 검증한다. 새로운 transport/provider보다 기존 capability의 계약·효과·증거 경계를 완결하는 것이 우선 판단 기준이다. 이 문서는 일정·작업 목록·완료 이력을 소유하지 않는다.
