# Prompt Audit Report

## 결론

- Catalog entries: **6**
- Catalog directives: **18**
- Catalog reference model payloads: **6**
- Runtime matrix contract: **6 × 2 languages = 12 cases**
- Reference text contracts: **0 sample-copy / 0 intentional blank-zone / 6 no-copy**
- Concrete catalog-owned test cases: **6**
- Validator mutation proofs: **5/5 detected**

모든 카테고리는 구조화 렌더러를 사용하며 지정된 조건을 자르지 않는다.

## 검수 표면

| Surface | Source of truth | Leakage boundary | Completeness contract | Structure boundary |
|---|---|---|---|---|
| Structured image compiler | catalog + typed request | no ID/path/hash/directive key/intent | every selected directive | semantic slot labels only |
| Codex image adapter | compiled prompt + output parameters | JSON data boundary | one prompt + all parameters | fixed execution contract owns headings |
| Catalog reference generator | catalog directives | metadata outside payload | every directive exactly once | fenced payload only |

## Mutation proof

| Defect class | Deliberate mutation | Detected |
|---|---|---:|
| leakage | inject entry.source_path | PASS |
| completeness | remove/duplicate a directive and remove catalog-owned audit copy | PASS |
| language | inject untranslated Hangul into English payload | PASS |
| structure | append an empty label and inject an authoring-mode directive | PASS |
| determinism | change the second render | PASS |

## Entry-by-entry projection matrix

| ID | Tier 2 purpose | Directives | Reference text | Structured | Generated payload |
|---|---|---:|---|---|---|
| C1 | 인물·사진 | 3 | none | all directives | audited |
| C4 | 제품·브랜드 | 3 | none | all directives | audited |
| C5 | 광고·포스터 | 3 | none | all directives | audited |
| C6 | 정보·UI | 3 | none | all directives | audited |
| C10 | 일러스트·스토리 | 3 | none | all directives | audited |
| C11 | 컨셉·엔터테인먼트 | 3 | none | all directives | audited |

## 증거

- Runtime exhaustive test: `crates/promptgen-core/src/image/render.rs::every_catalog_entry_compiles_to_a_clean_prompt`
- Profile validation: `crates/promptgen-core/src/image/validate.rs::validate_render_profile`
- Catalog authoring-source and payload audit: `scripts/audit-prompts.py`
- Deterministic payload generator: `scripts/generate-catalog-image-prompts.py`
- Synchronized renewal gate: `scripts/renew-image-catalog.py --check`
