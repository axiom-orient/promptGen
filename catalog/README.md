# 이미지 카탈로그 v5

`image_catalog.json`은 여섯 대표 결과물과 실행 directive의 정본이다. core는 빌드 시 파일을 포함해 검증·cache한다. 실행 중 카탈로그 PNG의 admission은 별도 Web 경계이며 [Web 분석](../crates/promptgen-web/ANALYSIS.md)이 owner다.

| ID | 결과물 | 범위 |
|---|---|---|
| C1 | 인물·라이프스타일 | portrait·fashion·profile·lifestyle |
| C4 | 제품·브랜드 | 제품 hero·package·brand·material |
| C5 | 광고·포스터 | campaign·poster·social banner |
| C6 | 정보·UI | infographic·diagram·presentation·UI mockup |
| C10 | 일러스트·스토리 | illustration·comic·sticker·storyboard |
| C11 | 컨셉·엔터테인먼트 | concept art·cinematic key art·game world |

## 계약·원본·생성물

`schema_version=5`, entries는 위 여섯 ID이며 모두 `tier_1=결과물`이다. 각 entry는 실행 directive 세 개와 PNG asset path 하나를 가진다. 요청은 `taxonomy.category` 하나로 결과물을 선택한다. `tier_2`는 Studio 탐색 묶음이고 별도 실행 선택값이 아니다. 스타일은 medium·lighting·palette·surface·composition·text 필드에 기술한다.

| 파일 | owner·성격 |
|---|---|
| [image_catalog.json](image_catalog.json) | taxonomy/directive 원본 |
| [reference_scenarios.json](reference_scenarios.json) | outcome별 대표 brief 원본 |
| `assets/category/*.png` | 대표 카드 여섯 개 |
| [CHATGPT_IMAGE_CATALOG_MANIFEST.json](CHATGPT_IMAGE_CATALOG_MANIFEST.json) | PNG byte/hash/dimension |
| [REPRESENTATIVE_IMAGE_QA.json](REPRESENTATIVE_IMAGE_QA.json) | PNG hash에 결합한 수동 criterion 기록 |
| [IMAGE_GENERATION_PROMPTS.md](IMAGE_GENERATION_PROMPTS.md) | 정본에서 만든 모델 payload |
| [IMAGE_CATALOG_ANALYSIS.md](IMAGE_CATALOG_ANALYSIS.md) | 카드별 시각 검토 근거 |

분류 근거는 [ADR 001](../docs/adr/001-outcome-taxonomy.md), 제품 방향은 [정체성과 발전](../docs/IDENTITY_AND_EVOLUTION.md), 현재 검증은 [구현 현황](../docs/IMPLEMENTATION_STATUS.md)에 있다.

## 변경·검증

카탈로그를 수정하면 파생 문서를 다시 만들고 같은 audit를 실행한다.

```sh
python3 scripts/renew-image-catalog.py
python3 scripts/audit-catalog-assets.py --write
python3 scripts/renew-image-catalog.py --check
python3 scripts/generate-catalog-image-prompts.py --check
python3 scripts/audit-prompts.py --check
python3 scripts/audit-catalog-assets.py
python3 scripts/audit-representative-image-qa.py
```

asset manifest audit는 파일 무결성을, QA audit는 기록의 구조와 hash 결합을 확인한다. 검사 통과만으로 모델 실행이나 새 이미지의 시각 품질을 주장하지 않는다.
