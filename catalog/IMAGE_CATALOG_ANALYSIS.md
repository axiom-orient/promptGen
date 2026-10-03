# Representative image catalog analysis

Reviewed 2026-07-30. This is visual QA for the six Studio reference cards, not proof that a compiled runtime prompt was transported to an external image model. Runtime fidelity requires a prompt/artifact receipt and is tracked separately in `evals/dogfood/current/`.

| ID | Asset | Observed result | Hard-contract review |
|---|---|---|---|
| C1 | `assets/category/people-lifestyle.png` | One adult East Asian furniture designer sketches a chair in a natural workshop. | One person, readable action, believable skin/fabric/wood response, no visible text. |
| C4 | `assets/category/product-brand.png` | One compact speaker and one matching package on a neutral product stage. | Exactly one product and one package; forms and material differences remain legible; no logo or text. |
| C5 | `assets/category/marketing-poster.png` | One beverage can with three citrus slices in a high-contrast campaign composition. | Exactly one can and three slices; poster hierarchy and copy-safe space are visible; no rendered text. |
| C6 | `assets/category/information-ui.png` | A restrained information-interface composition with six plotted data points. | The accepted revision has exactly six points, one reading order, consistent component language, and no accidental labels. |
| C10 | `assets/category/illustration-story.png` | A red-scarf fox completes one story across four panels. | Exactly four panels and one fox per panel; scarf, proportions, palette, and left-to-right continuity are stable. |
| C11 | `assets/category/concept-entertainment.png` | One explorer faces a monumental ancient ruin. | One focal character, clear environment scale, cinematic depth, and no visible franchise marks or text. |

All six files are opaque, non-interlaced 1254×1254 PNGs and are byte-bound to `CHATGPT_IMAGE_CATALOG_MANIFEST.json`.

## Regeneration record

The previous broad gallery was removed because it mixed output type, style, movement, mood, and editorial look in one navigation model. Six new cards were generated from outcome-specific briefs. C6 required two focused edits before the requested six-point count was met; the first candidates contained seven and five points. The accepted C10 card met its four-panel identity contract on the first reviewed candidate.

## Residual limits

- A visually counted reference card is not a model guarantee. Exact object count, exact text, and recurring identity still need post-generation checks.
- The catalog cards demonstrate decision boundaries, not every style. Users express style through typed fields.
- Adding a seventh outcome requires evidence that the request cannot be represented by an existing outcome plus typed fields.
