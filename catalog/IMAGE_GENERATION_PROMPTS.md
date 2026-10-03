# Catalog Image Generation Prompts

6개의 runtime catalog entry마다 하나씩 대응하는 모델 입력 payload다.
카탈로그 ID·경로·탐색 정보는 사람이 확인하는 metadata이며, 모델에는 각 fenced payload 내부만 전달한다.
이 문서는 `catalog/image_catalog.json`을 유일한 원천으로 결정론적으로 렌더한다.

## C1 — 인물·라이프스타일

- Asset: `catalog/assets/category/people-lifestyle.png`
- Navigation: 결과물 → 인물·사진
- Directive count: 3

### Model payload
<!-- MODEL_PAYLOAD_BEGIN:C1 -->
```text
Create exactly one polished visual reference image.
Treat every rule below as an observable image constraint, not as copy to render.

Catalog-owned reference test case:
- Create one environmental lifestyle portrait of a fictional adult furniture designer sketching at a sunlit studio table. Keep one person, one clear action, realistic skin and fabric, and no readable text.

Visual requirements:
- Subject rule: Keep each person's stated identity, age range, body proportions, face geometry, hair, wardrobe, pose, gaze, and action mutually consistent.
- Camera rule: Use a camera distance and viewpoint that make the person, action, and environmental context readable without accidental cropping or perspective distortion.
- Lighting rule: Preserve believable skin, hair, fabric, and environmental light response with clear subject separation and natural tonal detail.
- Canvas rule: use 4:5 as the primary aspect ratio.
- Palette rule: derive a restrained, category-appropriate palette from the visual requirements.
- Text rule: Do not add text, logos, watermarks, UI, or pseudo-letters.
- Safety rule: Keep people, if any, fictional adults and keep the scene suitable for a creative brief.
- Delivery rule: deliver one finished image only, with no unrelated collage, contact sheet, frame, or application chrome.
```
<!-- MODEL_PAYLOAD_END:C1 -->

## C4 — 제품·브랜드

- Asset: `catalog/assets/category/product-brand.png`
- Navigation: 결과물 → 제품·브랜드
- Directive count: 3

### Model payload
<!-- MODEL_PAYLOAD_BEGIN:C4 -->
```text
Create exactly one polished visual reference image.
Treat every rule below as an observable image constraint, not as copy to render.

Catalog-owned reference test case:
- Create one premium studio product image with exactly one fictional portable speaker and one matching unprinted package. Make polymer, woven fabric, aluminum, and recycled paper visibly distinct.

Visual requirements:
- Subject rule: Keep the product silhouette, dimensions, controls, openings, label placement, and functional parts coherent across every visible view.
- Composition rule: Establish one unmistakable hero view and give every secondary view, callout, package surface, or brand application one noncompeting role.
- Material rule: Differentiate each declared material and finish through physically plausible edge behavior, texture scale, reflection, refraction, and contact shadow.
- Canvas rule: use 1:1 as the primary aspect ratio.
- Palette rule: derive a restrained, category-appropriate palette from the visual requirements.
- Text rule: Do not add text, logos, watermarks, UI, or pseudo-letters.
- Safety rule: Keep people, if any, fictional adults and keep the scene suitable for a creative brief.
- Delivery rule: deliver one finished image only, with no unrelated collage, contact sheet, frame, or application chrome.
```
<!-- MODEL_PAYLOAD_END:C4 -->

## C5 — 광고·포스터

- Asset: `catalog/assets/category/marketing-poster.png`
- Navigation: 결과물 → 광고·포스터
- Directive count: 3

### Model payload
<!-- MODEL_PAYLOAD_BEGIN:C5 -->
```text
Create exactly one polished visual reference image.
Treat every rule below as an observable image constraint, not as copy to render.

Catalog-owned reference test case:
- Create one bold citrus-drink campaign poster with exactly one unbranded can, three citrus slices, one dynamic water arc, and a visibly empty copy-safe zone. Add no readable text.

Visual requirements:
- Composition rule: Build the campaign around one dominant hero and one clear reading path, with controlled negative space for supplied copy or channel-safe cropping.
- Typography rule: When exact copy is supplied, preserve it verbatim and separate headline, support line, metadata, and call to action through scale and alignment.
- Constraint rule: Keep brand cues and supporting elements subordinate to one message that remains legible and recognizable at thumbnail size.
- Canvas rule: use 4:5 as the primary aspect ratio.
- Palette rule: derive a restrained, category-appropriate palette from the visual requirements.
- Text rule: Keep typography optional. Do not invent copy, pseudo-text, logos, or glyphs; reserve a clean text-safe area only when the hierarchy benefits from it.
- Safety rule: Keep people, if any, fictional adults and keep the scene suitable for a creative brief.
- Delivery rule: deliver one finished image only, with no unrelated collage, contact sheet, frame, or application chrome.
```
<!-- MODEL_PAYLOAD_END:C5 -->

## C6 — 정보·UI

- Asset: `catalog/assets/category/information-ui.png`
- Navigation: 결과물 → 정보·UI
- Directive count: 3

### Model payload
<!-- MODEL_PAYLOAD_BEGIN:C6 -->
```text
Create exactly one polished visual reference image.
Treat every rule below as an observable image constraint, not as copy to render.

Catalog-owned reference test case:
- Create one responsive analytics interface shown on a desktop monitor and a phone. The desktop hero chart must contain exactly six data points; use only geometric icons, charts, and blank bars, with no pseudo-writing.

Visual requirements:
- Composition rule: Arrange the requested sections, states, steps, controls, or data groups in one explicit reading order with visible hierarchy and adequate spacing.
- Typography rule: Use discrete title, heading, label, annotation, and data scales so every supplied fact has one clear semantic level.
- Constraint rule: Keep each icon, connector, chart mark, and interface control tied to one meaning and inside the intended canvas or device boundary.
- Canvas rule: use 16:9 as the primary aspect ratio.
- Palette rule: derive a restrained, category-appropriate palette from the visual requirements.
- Text rule: Keep typography optional. Do not invent copy, pseudo-text, logos, or glyphs; reserve a clean text-safe area only when the hierarchy benefits from it.
- Safety rule: Keep people, if any, fictional adults and keep the scene suitable for a creative brief.
- Delivery rule: deliver one finished image only, with no unrelated collage, contact sheet, frame, or application chrome.
```
<!-- MODEL_PAYLOAD_END:C6 -->

## C10 — 일러스트·스토리

- Asset: `catalog/assets/category/illustration-story.png`
- Navigation: 결과물 → 일러스트·스토리
- Directive count: 3

### Model payload
<!-- MODEL_PAYLOAD_BEGIN:C10 -->
```text
Create exactly one polished visual reference image.
Treat every rule below as an observable image constraint, not as copy to render.

Catalog-owned reference test case:
- Create exactly four equal story panels. Show the same original red fox once per panel discovering a seed, planting it, waiting in rain, and greeting a sprout; preserve the scarf, satchel, face, and proportions.

Visual requirements:
- Subject rule: Lock recurring character or object identity before varying pose, expression, action, viewpoint, or narrative beat.
- Composition rule: Give every panel, frame, sticker, view, or scene one distinct role while preserving a clear sequence and consistent scale.
- Constraint rule: Preserve body proportions, face geometry, costume, props, line language, and palette roles across every repeated appearance.
- Canvas rule: use 2:3 as the primary aspect ratio.
- Palette rule: derive a restrained, category-appropriate palette from the visual requirements.
- Text rule: Do not add text, logos, watermarks, UI, or pseudo-letters.
- Safety rule: Keep people, if any, fictional adults and keep the scene suitable for a creative brief.
- Delivery rule: deliver exactly one finished sheet image. The requested grid, views, or panels are the artifact itself, not unrelated collage content; use no application chrome.
```
<!-- MODEL_PAYLOAD_END:C10 -->

## C11 — 컨셉·엔터테인먼트

- Asset: `catalog/assets/category/concept-entertainment.png`
- Navigation: 결과물 → 컨셉·엔터테인먼트
- Directive count: 3

### Model payload
<!-- MODEL_PAYLOAD_BEGIN:C11 -->
```text
Create exactly one polished visual reference image.
Treat every rule below as an observable image constraint, not as copy to render.

Catalog-owned reference test case:
- Create one cinematic wide concept scene of a solitary fictional explorer entering an immense flooded stone ruin. Make the scale, path, atmosphere, and focal light readable without titles or credits.

Visual requirements:
- Scene rule: Make the stated world, period, location, weather, and practical sources mutually consistent and visible in the environment.
- Composition rule: Build one readable hero silhouette across foreground, hero plane, and environmental depth instead of combining unrelated spectacle.
- Lighting rule: Use one motivated genre lighting grammar with coherent shadow direction, atmosphere, color separation, and material response.
- Canvas rule: use 16:9 as the primary aspect ratio.
- Palette rule: derive a restrained, category-appropriate palette from the visual requirements.
- Text rule: Do not add text, logos, watermarks, UI, or pseudo-letters.
- Safety rule: Keep people, if any, fictional adults and keep the scene suitable for a creative brief.
- Delivery rule: deliver one finished image only, with no unrelated collage, contact sheet, frame, or application chrome.
```
<!-- MODEL_PAYLOAD_END:C11 -->
