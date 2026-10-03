# promptGen 디자인 계약

제품 의미는 [SPEC](SPEC.md), 흐름은 [IMAGE_STUDIO_UX_SPEC](IMAGE_STUDIO_UX_SPEC.md)이 소유한다.
이 문서는 Studio asset contract와 prompt render 테스트가 참조하는 사용자 표현의 불변조건을 소유한다.
테스트 존재는 실제 브라우저 접근성 인증이 아니다. 실행 상태는 Current에 둔다.

## 2.1 의미 색상 토큰

light/dark를 모두 지원하는 컴포넌트는 surface/text/accent/skeleton 등 의미 토큰을 사용한다.
한 테마에서만 보이는 색상을 컴포넌트에 직접 고정하지 않는다.

## 4.3 카탈로그 탐색

전체 결과물·검색·이미지 preview로 여섯 outcome을 탐색할 수 있어야 한다. 중복 목적 filter와 선택 tray를 기본 작업 화면에 쌓지 않는다.
카드 보기와 category/profile의 확정 선택을 구분하며 profile/category 의미는 core가 검증한다.

## 4.4 키보드와 preview

preview focus containment, catalog keyboard navigation, 선택 시 의도하지 않은 scroll 방지를 유지한다.
이미지 비율에 맞는 card media frame을 사용하며 preview UI가 typed route를 암묵적으로 바꾸지 않는다.

## 5 typed control 보존

control 표현을 바꾸어도 typed request가 허용하는 aspect ratio·설정값을 UI에서 잃지 않는다.
unknown/incompatible 값은 조용히 다른 값으로 대체하지 않는다.

## 6.4 알림

polite live region은 하나를 사용하고 사용자 알림을 같은 announcer로 보낸다.
중복 알림을 만들지 않으며 API/provider의 내부 오류 문자열을 그대로 사용자 문구에 노출하지 않는다.

## 7 보조기술 상태

상태는 시각 효과뿐 아니라 ARIA 의미로 제공한다. keyboard/focus/reduced-motion 등 관련
규범은 실제 UX 계약에 맞추고 브라우저·보조기술 검증을 별도로 수행한다.

## 8 사용자 프롬프트와 내부 정보

compiled prompt에는 category/profile의 유효한 시각 의도를 전달한다. provenance·implementation
용어·내부 recipe ID를 생성 모델이 수행할 내용처럼 섞지 않는다. 해당 정보는 inspector에 둔다.
참조 이미지 설명으로 숨은 파일 경로를 발명하지 않는다.
