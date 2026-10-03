# 이미지 프롬프트 리뉴얼 근거 — 2026-09-30

기준 문서는 사용자가 지정한 `/Users/ax/Downloads/threads-image-prompt-research-improved.md`다. 입력 SHA-256은 `b88af74e66859852111d7a630a103820853e48ae5b6f76a83c396c1fff06053c`다. 원본은 수정하지 않았다. 후속 지시에 따라 SNS 원문·이미지 추가 열람을 중단하고 문서와 공식 문서·GitHub·디자인·생성 사이트를 대조했다.

## 채택 기준

- 신규 사례의 시점은 2026년 9월로 제한한다. 저장소 commit 날짜, 원 게시물 날짜, 모델 출시 후 검증 여부를 별개로 확인한다.
- 현재 공식 API 문서는 조회일을 기록하고 실제 지원 모델·매개변수만 코드에 반영한다. 조회일을 발행일로 바꾸지 않는다.
- 날짜 없는 템플릿 사이트는 현재 구성의 관찰 자료이며 최신 출시나 이미지 품질의 증거가 아니다.
- 프롬프트·입력·출력·모델·설정이 함께 확인되지 않으면 검증된 생성 사례로 세지 않는다.
- 외부 프롬프트 전문과 이미지 자산을 복제하지 않는다. 아래 어휘와 예시는 이 제품의 계약에 맞춰 새로 작성했다.

## 확인한 현재 자료와 적용 판단

| 자료 | 시점과 증거 | 이번 적용 |
|---|---|---|
| [OpenAI 이미지 prompting guide](https://developers.openai.com/api/docs/guides/image-prompting) | 9/30 공식 본문 조회; GPT Image 2.5 가이드 | 대상·구도·재질·빛·정확 문구·변경/보존을 분리하고 결과를 검수 |
| [OpenAI 이미지 generation guide](https://developers.openai.com/api/docs/guides/image-generation) | 9/30 공식 본문 조회 | API 모델·품질·크기 제어를 조사한 자료; 구독 도구의 제어 인자로 적용하지 않음 |
| [Sunburst 모델 카드](https://developers.openai.com/api/docs/models/gpt-image-2.5-sunburst) | 9/30 공식 본문 조회 | API 연구 대상으로 조사했으며 현재 구독 runtime의 모델 식별자로 사용하지 않음 |
| [Flare 모델 카드](https://developers.openai.com/api/docs/models/gpt-image-2.5-flare) | 9/30 공식 본문 조회 | 비교 대상으로 조사. 이 제품에 숨은 모델 전환·fallback을 추가하지 않음 |
| [Google 현재 이미지 생성 문서](https://ai.google.dev/gemini-api/docs/image-generation) | 9/30 현재 Nano Banana 문서 조회 | 참조의 역할과 이전 결과를 명확히 지정하는 원칙을 교차 확인; Gemini 실행 adapter 추가 없음 |
| [Midjourney Edit Model](https://docs.midjourney.com/hc/en-us/articles/48495453462797-Edit-Model) | 9/30 현재 문서 조회 | 참조 기반 생성·부분 편집 구별; 고유 파라미터를 공통 프롬프트에 섞지 않음 |
| [Midjourney Version](https://docs.midjourney.com/hc/en-us/articles/32199405667853-Version) | 9/30 현재 V8.2 문서 조회 | 예전 Omni Reference 관례를 현행 공통 API 규칙으로 사용하지 않음 |
| [AtlasCloudAI collection](https://github.com/AtlasCloudAI/awesome-gpt-image-2.5-prompts/tree/e58b4e1fac09eca0ba491b09cea0bf061da688d1) | commit 9/30 `e58b4e1`; README·작성자/입력 설명 확인 | 참조·문구·편집·기하·연속성별 검수 항목을 조사. 제품에서 재현했다고 주장하지 않음 |
| [VulcanEon collection](https://github.com/VulcanEon/awesome-gpt-image-2.5-prompts/tree/27a4e330fdbe936ef4739eac984a0130446bbaff) | commit 9/14; 공식 companion 9/13 | 30 visual studies·20 editing recipes·24 tutorial steps를 구별. 자체 rewritten prompt는 독립 재현되지 않았다고 명시하므로 품질 증명에서 제외 |
| [opensource-works collection](https://github.com/opensource-works/awesome-gpt-image-prompts/tree/a1fd4fe6c5be684bc06d801f371c56f45be022d5) | commit 9/28; 개별 source date 포함 | 캐릭터 시트·장소 변경·정확 문구의 실패 축을 비교. 이전 게시물을 최신 실행으로 세지 않음 |
| [youart collection](https://github.com/youart-open-source/awesome-gpt-image-2-5-prompts/tree/f7f6276ff3d82cea2d837b89cf2dd097dd9766ac) | commit 9/11; README에서 149개 dated posts 중 144개가 2.5 출시 전이라고 명시 | 최신 모델에서 재현된 corpus로 채택하지 않음. 이름·갱신일만으로 최신 증거를 판정하지 않는 반례 |
| [Figma generative tools](https://www.figma.com/blog/how-we-built-generative-plugins-and-shaders/) | 발행 9/1 | 동작하는 제어와 결과를 같은 작업 맥락에서 수정하는 관점. 프롬프트를 접힌 상세에서 작업 면으로 이동 |
| [Canva 현재 scrapbook catalogue](https://www.canva.com/scrapbooks/templates/modern/) | 9/30 현재 catalogue 관찰; 개별 발행일 UNKNOWN | 사진·종이·문구를 분리하는 구성 참고. 생성 모델 성능 증거로 사용하지 않음 |
| [Adobe/Topaz 현재 발표](https://blog.adobe.com/en/publish/2026/09/23/adobe-completes-acquisition-of-topaz-labs) | 발행 9/23 | 업스케일·복원과 최초 생성 품질을 구별. 업스케일러를 붙였다는 이유로 원본 정합성 통과 처리하지 않음 |

추가 탐색에서 나온 Adobe 6–8월 안내, 3월 FLUX.2 repository commit, 8월 slash-command 저장소는 이번 **신규 최신 사례 집합에서 제외**했다. 현재 BFL 문서의 구조화 표현도 확인했지만 해당 규칙을 새 모델의 검증된 성능으로 가져오지 않았다. 검색 결과 수, stars, 조회수는 품질 점수가 아니다.

## 문서의 요구를 실행 계약으로 변환

| 사용자 작업 | 명확하게 지정할 것 | 고정할 것 | 검수할 것 |
|---|---|---|---|
| 여행 사진에 메모 | 메모 문구·언어·위치·펜 획 | 얼굴·옷·자세·사진 내부 구도·장소 | 글자 정확성, 얼굴 가림, 원본 변화 |
| 여행 scrapbook | 중심 장면, 종이 조각, 테이프와 겹침 | 중심 이미지의 식별 요소·읽기 순서 | 사진이 주인공인지, 장식 글자·날짜가 발명됐는지 |
| 사진+인쇄물 | 사진 영역과 인쇄 질감 영역 | 사진 영역의 원근·빛·색 | 스타일이 사진 내부로 번졌는지 |
| 여행 계획 기록장 | 제공한 장소·날짜·이동 순서 | 제공 사실과 순서 | 미확인 주소·운영시간·이동시간 추가 여부 |
| 브랜드·제품 | 형상·부품 수·재질·색 역할 | 실루엣·라벨·크기 관계 | 추가 부품·잘못된 연결·철자 |
| 인물 연속성 | 식별 특징·의상·행동·시선 | 기준 인물과 상대 비율 | 얼굴·헤어·포즈·좌우 반전 변화 |
| 여러 패널 | 칸 수·읽기 순서·각 행동 | 동일 인물·색·공간 규칙 | 누락/중복 칸, 다른 인물, 비인과적 순서 |
| 기술 도해 | 실제 구성 요소와 연결 | 부품 수·연결 방향·축척 | 숨은 부품·라벨·치수 발명 |

### 일관성의 다섯 층

1. **의미:** 주 대상, 수량, 관계, 실제로 전달한 참조가 같다.
2. **기하:** 배치, 비율, 카메라 방향, 손·발·접촉이 모순되지 않는다.
3. **표현:** 재질, 색 역할, 광원 방향, 종이/사진 영역이 일치한다.
4. **문구·사실:** 지정 문구와 줄바꿈을 보존하고 미지정 지명·날짜·숫자를 만들지 않는다.
5. **실행 증거:** canonical prompt, review, executed prompt, 실제 PNG와 receipt를 해시로 연결한다.

같은 프롬프트를 얻는 결정성과 같은 이미지를 다시 얻는 재현성은 다르다. 참조 입력·설정·반복 결과를 관찰하지 않고 캐릭터 일관성이나 픽셀 동일성을 보장하지 않는다. 픽셀 보존이 필요한 최종 작업은 보호 영역을 실제 합성 과정에서 유지하고 따로 검증해야 한다.

## 관찰 가능한 어휘

단축어는 제품의 authoring shorthand다. provider에 slash 문자열을 전달하지 않고 기존 typed 필드로 확장한다. JSON이라는 형식 자체가 높은 품질을 보장하는 것도 아니다.

| 축 | 지원 어휘 | 실제 의미 |
|---|---|---|
| 기록·배치 | `/travel-journal`, `/scrapbook` | 제공 사실만 쓰는 기록, 주 장면을 가리지 않는 비대칭 종이 콜라주 |
| 표현·문구 | `/risograph`, `/handwritten` | 제한된 색판·인쇄 입자, 확정 문구의 판독 가능한 펜 획 |
| 시점 | `/isometric`, `/topdown`, `/lowangle`, `/highangle` | 투영·시선 높이·카메라 방향을 명시 |
| 프레이밍 | `/macro`, `/closeup`, `/headshot`, `/fullbody`, `/wideshot` | 피사체 범위와 프레임에 포함할 부분을 명시 |
| 초점 | `/shallowdepth`, `/deepfocus` | 선명할 영역과 흐릴 영역을 구분 |
| 내부 구조 | `/cutaway`, `/explodedview` | 지정 부품의 절개 또는 조립 축 분리; 숨은 부품 발명 금지 |
| 매체 | `/watercolor`, `/ink`, `/clay`, `/blueprint` | 안료·선·점토 표면·기술선의 관찰 가능한 표현 |
| 조명 | `/softlight`, `/goldenhour` | 확산광 또는 낮은 따뜻한 태양광을 typed 조명에 연결 |

같은 축의 경쟁 지시, 미지원 단축어, 매체/조명과의 충돌은 거절한다. `/travel-journal /scrapbook`은 목적과 배치가 달라 조합할 수 있다. `/macro /deepfocus`는 현재 계약의 초점 요구가 달라 함께 쓰지 않는다. 문구 안의 slash, 본문의 URL은 명령으로 해석하지 않는다.

인물·상품을 보존하는 edit에서는 카메라/광원 교체 단축어를 허용하지 않는다. 수채화·잉크·리소그래프는 **지정된 추가 영역에만** 적용하며 원본 인물의 특징으로 넣지 않는다. 광원·색온도·LUT를 추정해 원본 사진에 덮어씌우지 않는다.

## 새로 작성한 짧은 입력 예시

외부 게시물 전문의 복사가 아니다. 지명과 문구는 예시 입력이며 실제 여행 사실로 주장하지 않는다.

```text
/travel-journal /scrapbook /handwritten
제주 해변 여행 기록 한 장. 수평선 아래 푸른 바다, 크림색 종이 여백.
제목은 "바람을 따라". 다른 문구나 날짜 없음.
```

```text
/scrapbook
원본 사진의 얼굴·옷·자세·장소와 사진 내부 구도를 유지한다.
사진 바깥 여백에 얇은 테이프와 크림색 종이만 더한다.
문구는 "오늘의 작은 기록"만 사용하고 날짜나 장소명을 추가하지 않는다.
```

```text
/risograph
작은 독립 책방의 안내 포스터. 검정과 붉은 색판, 종이 결, 넓은 여백.
문구는 "책과 함께" 한 줄. 다른 라벨·가격·주소 없음.
```

```text
/isometric /cutaway
작은 온실의 관수 구조. 물탱크·펌프·배관·노즐·재배대만 포함.
한쪽 외벽 일부를 제거해 실제 연결 순서를 드러낸다. 문구·치수 없음.
```

```text
/macro /softlight
무광 황동 시계 1개. 열린 기어와 미세한 긁힘.
왼쪽 확산광, 짙은 갈색 천 배경. 문구 없음.
```

```text
/fullbody
성인 인물 1명이 바닥에 두 발을 딛고 서 있다. 흰 셔츠와 짙은 청바지.
머리와 발끝을 모두 포함. 시선은 오른쪽 창문. 문구 없음.
```

## 코드에 연결한 것

- C5의 `travel_journal` 프로필: 광고 CTA·브랜드 지시와 분리된 사진·종이·메모 계약. 새 카드나 별도 renderer를 만들지 않는다.
- `image/controls.rs`: 23개 어휘, 경쟁 축·매체 검증, 기존 domain 슬롯에 확장.
- `interview/image.rs`: 정확 문구 재사용, 카메라·조명 충돌 거절, generate와 단일 base edit 구분.
- source edit: 원본 역할을 선언하고 이미 제공한 보존·변경·문구를 재사용한다. 실제 파일 binding은 Codex 구독 adapter가 소유.
- 실행 원본은 core의 `IMAGE_BACKEND=codex-subscription` 하나다. `detail=high`는 프롬프트의 묘사 의도이며 API 모델/품질 제어가 아니다.
- `editorial_flat`과 길이 예산으로 요구를 잘라내는 renderer를 폐기. 모든 일반 작업은 조건을 보존하는 renderer를 사용.
- Studio: 입력과 실제 프롬프트를 나란히 표시. 설정·검수·provenance는 필요할 때 열며 내부 backend 용어를 기본 작업 화면에서 줄임.

## 검증의 한계

새 어휘·서버 배선·컴파일 계약의 테스트와 실제 Studio 조작은 [Current](IMPLEMENTATION_STATUS.md)에 기록한다. 이 조사에서 source–prompt–output 재현 실험으로 새 이미지 모델의 시각 품질을 인증하지 않았다. `max`는 요청 설정이며 정합성 합격증이 아니다. 모델별 가격·지연 시간을 측정하거나 보장하지 않았다.
