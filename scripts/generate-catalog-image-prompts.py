#!/usr/bin/env python3
"""Render one metadata-separated model payload for every runtime catalog entry."""

from __future__ import annotations

import argparse
import json
import pathlib
import sys
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parent.parent
CATALOG = ROOT / "catalog" / "image_catalog.json"
SCENARIOS = ROOT / "catalog" / "reference_scenarios.json"
OUTPUT = ROOT / "catalog" / "IMAGE_GENERATION_PROMPTS.md"
SHEET_ENTRY_IDS = {"C10"}
SLOT_LABELS = {
    "subject": "Subject rule",
    "scene": "Scene rule",
    "composition": "Composition rule",
    "camera": "Camera rule",
    "lighting": "Lighting rule",
    "color": "Color rule",
    "material": "Material rule",
    "typography": "Typography rule",
    "motion": "Motion rule",
    "constraint": "Constraint rule",
}


def text_rule(entry: dict[str, Any]) -> str:
    mode = entry["reference_text_mode"]
    exact_lines = entry["reference_exact_text"]
    if mode == "sample_copy":
        rendered_lines = "\n".join(
            f"  - Exact text line {index}: {json.dumps(line, ensure_ascii=False)}"
            for index, line in enumerate(exact_lines, 1)
        )
        usage = (
            "Repeat only this exact copy where a visual directive explicitly requires repetition."
            if entry["repetition_text"]
            else "Render each exact line once and preserve the listed line order."
        )
        return (
            "Render only the catalog-owned audit copy below; preserve spelling and capitalization "
            "exactly. Do not add other readable text, logos, pseudo-text, or watermarks.\n"
            f"{rendered_lines}\n"
            f"  - Copy usage: {usage}"
        )
    if mode == "blank_zone" and not exact_lines:
        return (
            "This reference intentionally validates blank copy-zone geometry. Do not invent copy, "
            "pseudo-text, logos, or glyphs; keep the specified copy zone visibly empty."
        )
    if mode == "none" and not exact_lines:
        if entry["text_requirement"] == "recommended":
            return (
                "Keep typography optional. Do not invent copy, pseudo-text, logos, or glyphs; "
                "reserve a clean text-safe area only when the hierarchy benefits from it."
            )
        return "Do not add text, logos, watermarks, UI, or pseudo-letters."
    raise ValueError(f"{entry['id']}: invalid reference text contract")


def safety_rule(tier: int) -> str:
    if tier == 2:
        return (
            "If a person is present, depict a fictional adult aged 25+ in fully opaque, non-nude "
            "clothing; never sexualize the subject."
        )
    return (
        "Keep people, if any, fictional adults and keep the scene suitable for a creative brief."
    )


def delivery_rule(entry_id: str) -> str:
    if entry_id in SHEET_ENTRY_IDS:
        return (
            "Delivery rule: deliver exactly one finished sheet image. The requested grid, views, or "
            "panels are the artifact itself, not unrelated collage content; use no application chrome."
        )
    return (
        "Delivery rule: deliver one finished image only, with no unrelated collage, contact sheet, "
        "frame, or application chrome."
    )


def model_payload(entry: dict[str, Any], scenarios: dict[str, str]) -> str:
    aspect_ratios = entry["suggested_aspect_ratios"]
    lines = [
        "Create exactly one polished visual reference image.",
        "Treat every rule below as an observable image constraint, not as copy to render.",
    ]
    scenario = scenarios.get(entry["id"])
    if scenario:
        lines.extend(["", "Catalog-owned reference test case:", f"- {scenario}"])
    lines.extend(["", "Visual requirements:"])
    for directive in entry["prompt_directives"]:
        slot = directive["slot"]
        try:
            label = SLOT_LABELS[slot]
        except KeyError as error:
            raise ValueError(f"unknown directive slot {slot!r}") from error
        lines.append(f'- {label}: {directive["text"]}')
    lines.extend(
        [
            f"- Canvas rule: use {aspect_ratios[0] if aspect_ratios else '1:1'} as the primary aspect ratio.",
            "- Palette rule: derive a restrained, category-appropriate palette from the visual requirements.",
            f"- Text rule: {text_rule(entry)}",
            f"- Safety rule: {safety_rule(entry['default_safety_tier'])}",
            f"- {delivery_rule(entry['id'])}",
        ]
    )
    return "\n".join(lines)


def render(catalog: dict[str, Any], scenarios: dict[str, str]) -> str:
    entries = catalog["entries"]
    lines = [
        "# Catalog Image Generation Prompts",
        "",
        f"{len(entries)}개의 runtime catalog entry마다 하나씩 대응하는 모델 입력 payload다.",
        "카탈로그 ID·경로·탐색 정보는 사람이 확인하는 metadata이며, 모델에는 각 fenced payload 내부만 전달한다.",
        "이 문서는 `catalog/image_catalog.json`을 유일한 원천으로 결정론적으로 렌더한다.",
        "",
    ]
    for entry in entries:
        entry_id = entry["id"]
        name = entry["name_ko"] or entry["name_en"]
        navigation = f"{entry['tier_1']} → {entry['tier_2']}"
        if entry.get("tier_3"):
            navigation += f" → {entry['tier_3']}"
        lines.extend(
            [
                f"## {entry_id} — {name}",
                "",
                f"- Asset: `catalog/{entry['asset_path']}`",
                f"- Navigation: {navigation}",
                f"- Directive count: {len(entry['prompt_directives'])}",
                "",
                "### Model payload",
                f"<!-- MODEL_PAYLOAD_BEGIN:{entry_id} -->",
                "```text",
                model_payload(entry, scenarios),
                "```",
                f"<!-- MODEL_PAYLOAD_END:{entry_id} -->",
                "",
            ]
        )
    return "\n".join(lines)


def load_catalog() -> dict[str, Any]:
    return json.loads(CATALOG.read_text(encoding="utf-8"))


def load_scenarios() -> dict[str, str]:
    data = json.loads(SCENARIOS.read_text(encoding="utf-8"))
    if data.get("format") != "promptgen-catalog-reference-scenarios/1":
        raise ValueError("unsupported reference scenario format")
    scenarios = data.get("scenarios")
    if not isinstance(scenarios, dict):
        raise ValueError("reference scenarios must be an object")
    for entry_id, value in scenarios.items():
        if not isinstance(entry_id, str) or not isinstance(value, str) or not value.strip():
            raise ValueError(f"invalid reference scenario: {entry_id!r}")
        if value != value.strip() or "\n" in value or "\r" in value:
            raise ValueError(f"reference scenario must be one canonical line: {entry_id}")
    return scenarios


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail when the checked-in output is stale")
    args = parser.parse_args()
    catalog = load_catalog()
    scenarios = load_scenarios()
    catalog_ids = {entry["id"] for entry in catalog["entries"]}
    unknown_ids = sorted(set(scenarios) - catalog_ids)
    if unknown_ids:
        raise ValueError(f"reference scenarios contain unknown catalog IDs: {unknown_ids}")
    rendered = render(catalog, scenarios)
    if args.check:
        if not OUTPUT.is_file() or OUTPUT.read_text(encoding="utf-8") != rendered:
            print("[FAIL] catalog image generation prompts are stale", file=sys.stderr)
            return 1
        print(f"[OK] catalog image generation prompts: {len(catalog['entries'])} entries")
        return 0
    OUTPUT.write_text(rendered, encoding="utf-8")
    print(f"[OK] wrote {OUTPUT}: {len(catalog['entries'])} entries")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
