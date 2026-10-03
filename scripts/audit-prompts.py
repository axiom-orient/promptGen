#!/usr/bin/env python3
"""Exhaustively audit catalog-derived model payloads and prove the auditor by mutation."""

from __future__ import annotations

import argparse
import copy
import importlib.util
import json
import pathlib
import re
import sys
from dataclasses import dataclass
from typing import Any, Iterable


ROOT = pathlib.Path(__file__).resolve().parent.parent
CATALOG_PATH = ROOT / "catalog" / "image_catalog.json"
PROMPTS_PATH = ROOT / "catalog" / "IMAGE_GENERATION_PROMPTS.md"
SCENARIOS_PATH = ROOT / "catalog" / "reference_scenarios.json"
REPORT_PATH = ROOT / "docs" / "PROMPT_AUDIT_REPORT.md"
GENERATOR_PATH = ROOT / "scripts" / "generate-catalog-image-prompts.py"
BEGIN_RE = re.compile(r"^<!-- MODEL_PAYLOAD_BEGIN:([A-Z0-9-]+) -->$")
END_RE = re.compile(r"^<!-- MODEL_PAYLOAD_END:([A-Z0-9-]+) -->$")
HANGUL_RE = re.compile(r"[\uac00-\ud7a3]")
EMPTY_LABEL_RE = re.compile(r"^-\s+[^:\n]+:\s*$")
IDENTIFIER_BOUNDARY = r"(?<![A-Za-z0-9-]){}(?![A-Za-z0-9-])"
VALID_SLOTS = {
    "subject", "scene", "composition", "camera", "lighting",
    "color", "material", "typography", "motion", "constraint",
}
AUTHORING_PREFIXES = (
    "describe ", "choose ", "define ", "declare ", "specify ", "state ",
    "decide ", "select ", "determine ", "articulate ", "explain ", "list ",
)
RATIO_TOKEN_RE = re.compile(r"(?<![0-9])(?:1:1|2:3|3:2|3:4|4:5|4:3|16:9|9:16)(?![0-9])")
EXPECTED_SHEET_IDS = {"C10"}
UNRESOLVED_REFERENCE_RE = re.compile(r"\b(requested|supplied|specified|provided|designated|chosen|selected)\b", re.IGNORECASE)


@dataclass(frozen=True)
class Violation:
    kind: str
    entry_id: str
    detail: str


def load_generator():
    spec = importlib.util.spec_from_file_location("promptgen_catalog_prompt_generator", GENERATOR_PATH)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot import {GENERATOR_PATH}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def parse_payloads(document: str) -> tuple[list[str], dict[str, str]]:
    lines = document.splitlines()
    order: list[str] = []
    payloads: dict[str, str] = {}
    index = 0
    while index < len(lines):
        begin = BEGIN_RE.match(lines[index])
        if begin is None:
            index += 1
            continue
        entry_id = begin.group(1)
        if entry_id in payloads:
            raise ValueError(f"duplicate model payload {entry_id}")
        if index + 1 >= len(lines) or lines[index + 1] != "```text":
            raise ValueError(f"{entry_id}: payload must start with a text fence")
        cursor = index + 2
        body: list[str] = []
        while cursor < len(lines) and lines[cursor] != "```":
            body.append(lines[cursor])
            cursor += 1
        if cursor >= len(lines):
            raise ValueError(f"{entry_id}: unterminated text fence")
        if cursor + 1 >= len(lines):
            raise ValueError(f"{entry_id}: missing end marker")
        end = END_RE.match(lines[cursor + 1])
        if end is None or end.group(1) != entry_id:
            raise ValueError(f"{entry_id}: mismatched end marker")
        payloads[entry_id] = "\n".join(body)
        order.append(entry_id)
        index = cursor + 2
    return order, payloads


def contains_identifier(payload: str, identifier: str) -> bool:
    if not identifier:
        return False
    return re.search(IDENTIFIER_BOUNDARY.format(re.escape(identifier)), payload, flags=re.IGNORECASE) is not None


def derived_leak_tokens(entry: dict[str, Any]) -> Iterable[tuple[str, str]]:
    yield "source_path", entry["source_path"]
    yield "source_sha256", entry["source_sha256"]
    yield "asset_path", entry["asset_path"]
    yield "name_ko", entry["name_ko"]
    name_en = entry["name_en"]
    if name_en:
        yield "name_en", name_en



def audit_catalog_entry(entry: dict[str, Any], scenarios: dict[str, str] | None = None) -> list[Violation]:
    """Audit the catalog authoring source before it is projected into payloads."""
    entry_id = entry.get("id", "<missing>")
    violations: list[Violation] = []
    directives = entry.get("prompt_directives")
    if not isinstance(directives, list) or len(directives) < 2:
        return [Violation("completeness", entry_id, "catalog entry needs at least two directives")]

    seen_text: set[str] = set()
    for index, directive in enumerate(directives, 1):
        if not isinstance(directive, dict) or set(directive) != {"slot", "text"}:
            violations.append(
                Violation("structure", entry_id, f"directive[{index}] must contain only slot and text")
            )
            continue
        slot = directive["slot"]
        text = directive["text"]
        if slot not in VALID_SLOTS:
            violations.append(Violation("structure", entry_id, f"directive[{index}] has unknown slot {slot!r}"))
        if not isinstance(text, str) or not text.strip():
            violations.append(Violation("completeness", entry_id, f"directive[{index}] text is empty"))
            continue
        if text != text.strip() or "\n" in text or "\r" in text:
            violations.append(Violation("structure", entry_id, f"directive[{index}] is not one canonical line"))
        if text in seen_text:
            violations.append(Violation("completeness", entry_id, f"directive[{index}] duplicates an earlier directive"))
        seen_text.add(text)

        lower = text.lower()
        if lower.startswith(AUTHORING_PREFIXES):
            violations.append(
                Violation("structure", entry_id, f"directive[{index}] starts with an authoring-mode verb")
            )
        if lower.startswith("ar ") or " default ar" in lower or "aspect ratio" in lower or RATIO_TOKEN_RE.search(text):
            violations.append(
                Violation("structure", entry_id, f"directive[{index}] overrides typed canvas authority")
            )
        if HANGUL_RE.search(text):
            violations.append(Violation("language", entry_id, f"directive[{index}] is not English"))
        if any(marker in text for marker in ("{", "}", "```", "MODEL_PAYLOAD_")):
            violations.append(Violation("structure", entry_id, f"directive[{index}] contains wrapper syntax"))
        if contains_identifier(text, entry_id) or f"directive_{entry_id}".lower() in lower:
            violations.append(Violation("leakage", entry_id, f"directive[{index}] leaks an implementation identifier"))
        for field in ("source_path", "source_sha256", "asset_path"):
            token = entry.get(field, "")
            if token and token.lower() in lower:
                violations.append(Violation("leakage", entry_id, f"directive[{index}] leaks {field}"))

    if scenarios is not None and any(
        UNRESOLVED_REFERENCE_RE.search(directive.get("text", ""))
        for directive in directives
        if isinstance(directive, dict)
    ):
        scenario = scenarios.get(entry_id)
        if not isinstance(scenario, str) or not scenario.strip():
            violations.append(
                Violation(
                    "completeness",
                    entry_id,
                    "reference payload contains supplied/requested slots but has no concrete catalog-owned test case",
                )
            )

    mode = entry.get("reference_text_mode")
    exact_text = entry.get("reference_exact_text")
    requirement = entry.get("text_requirement")
    if mode not in {"none", "blank_zone", "sample_copy"} or not isinstance(exact_text, list):
        violations.append(Violation("structure", entry_id, "invalid reference-text contract shape"))
    else:
        invalid_line = any(
            not isinstance(line, str) or not line.strip() or "\n" in line or "\r" in line
            for line in exact_text
        )
        if invalid_line:
            violations.append(Violation("structure", entry_id, "reference exact text has an empty or multiline value"))
        if mode == "none" and (exact_text or requirement == "required"):
            violations.append(Violation("completeness", entry_id, "required text has no explicit reference policy"))
        if mode == "blank_zone" and (exact_text or requirement != "required"):
            violations.append(Violation("structure", entry_id, "blank-zone reference-text contract is inconsistent"))
        if mode == "sample_copy" and (not exact_text or requirement != "required"):
            violations.append(Violation("completeness", entry_id, "sample-copy reference-text contract is incomplete"))

    return violations

def audit_payload(entry: dict[str, Any], payload: str, scenario: str | None = None) -> list[Violation]:
    entry_id = entry["id"]
    violations: list[Violation] = []

    # Leakage: derive the forbidden provenance/identity surface from this entry.
    if contains_identifier(payload, entry_id):
        violations.append(Violation("leakage", entry_id, f"entry id {entry_id!r} leaked"))
    if f"directive_{entry_id}".lower() in payload.lower():
        violations.append(Violation("leakage", entry_id, "directive implementation key leaked"))
    for field, token in derived_leak_tokens(entry):
        if token and token.lower() in payload.lower():
            violations.append(Violation("leakage", entry_id, f"{field} leaked: {token!r}"))
    leakage_surface = payload
    for exact_text in entry["reference_exact_text"]:
        leakage_surface = leakage_surface.replace(json.dumps(exact_text, ensure_ascii=False), "")
    if "promptgen" in leakage_surface.lower():
        violations.append(Violation("leakage", entry_id, "repository/product name leaked outside exact copy"))

    # Completeness: every selected directive must reach this generated reference payload once.
    for index, directive in enumerate(entry["prompt_directives"], 1):
        count = payload.count(directive["text"])
        if count != 1:
            violations.append(
                Violation(
                    "completeness",
                    entry_id,
                    f"directive[{index}] occurrence count is {count}, expected 1",
                )
            )

    for index, exact_text in enumerate(entry["reference_exact_text"], 1):
        quoted = json.dumps(exact_text, ensure_ascii=False)
        count = payload.count(quoted)
        if count != 1:
            violations.append(
                Violation(
                    "completeness",
                    entry_id,
                    f"reference_exact_text[{index}] occurrence count is {count}, expected 1",
                )
            )

    if scenario is not None:
        count = payload.count(scenario)
        if count != 1:
            violations.append(
                Violation(
                    "completeness",
                    entry_id,
                    f"reference scenario occurrence count is {count}, expected 1",
                )
            )

    # The instruction surface is English, but exact copy may intentionally use another script.
    language_surface = payload
    for exact_text in entry["reference_exact_text"]:
        language_surface = language_surface.replace(json.dumps(exact_text, ensure_ascii=False), "")
    if HANGUL_RE.search(language_surface):
        violations.append(Violation("language", entry_id, "English instruction surface contains Hangul outside exact copy"))

    # Structure: metadata and user-copy placeholders cannot forge model-owned structure.
    if payload.count("Visual requirements:") != 1:
        violations.append(Violation("structure", entry_id, "Visual requirements section count is not 1"))
    for line_number, line in enumerate(payload.splitlines(), 1):
        if EMPTY_LABEL_RE.match(line):
            violations.append(
                Violation("structure", entry_id, f"line {line_number} has a label with no value")
            )
        if line.startswith("#"):
            violations.append(
                Violation("structure", entry_id, f"line {line_number} forges a Markdown heading")
            )
    if "::" in payload:
        violations.append(Violation("structure", entry_id, "payload contains doubled separator '::'"))
    if any(marker in payload for marker in ("{", "}", "```", "MODEL_PAYLOAD_BEGIN", "MODEL_PAYLOAD_END")):
        violations.append(Violation("structure", entry_id, "payload contains placeholder or wrapper syntax"))
    if not payload.startswith("Create exactly one polished visual reference image.\n"):
        violations.append(Violation("structure", entry_id, "payload start contract changed"))
    expected_delivery = (
        "deliver exactly one finished sheet image"
        if entry_id in EXPECTED_SHEET_IDS
        else "deliver one finished image only"
    )
    if expected_delivery not in payload:
        violations.append(
            Violation("structure", entry_id, f"output-shape contract missing: {expected_delivery!r}")
        )
    if entry_id == "C10" and "exactly four" not in payload.lower():
        violations.append(Violation("completeness", entry_id, "four-panel invariant is missing"))

    return violations


def audit_document(catalog: dict[str, Any], scenarios: dict[str, str], document: str) -> tuple[list[Violation], dict[str, str]]:
    expected_ids = [entry["id"] for entry in catalog["entries"]]
    order, payloads = parse_payloads(document)
    violations: list[Violation] = []
    for entry in catalog["entries"]:
        violations.extend(audit_catalog_entry(entry, scenarios))
    if order != expected_ids:
        violations.append(
            Violation(
                "completeness",
                "$catalog",
                f"payload order/coverage differs: expected {expected_ids!r}, got {order!r}",
            )
        )
    for entry in catalog["entries"]:
        payload = payloads.get(entry["id"])
        if payload is None:
            violations.append(Violation("completeness", entry["id"], "model payload is missing"))
            continue
        violations.extend(audit_payload(entry, payload, scenarios.get(entry["id"])))
    return violations, payloads


def audit_determinism(first: str, second: str) -> list[Violation]:
    if first == second:
        return []
    return [Violation("determinism", "$generator", "two renders differ for identical input")]


def prove_mutations(catalog: dict[str, Any], payloads: dict[str, str]) -> dict[str, bool]:
    entry = catalog["entries"][0]
    entry_id = entry["id"]
    original = payloads[entry_id]
    first_directive = entry["prompt_directives"][0]["text"]
    mutations = {
        "leakage": [original + f"\n- Provenance: {entry['source_path']}"],
        "completeness": [
            original.replace(first_directive, "", 1),
            original + f"\n- Duplicate directive: {first_directive}",
        ],
        "language": [original + "\n- Language mutation: 번역되지 않은 원문"],
        "structure": [original + "\n- Empty label:"],
    }
    results: dict[str, bool] = {}
    for kind, variants in mutations.items():
        detected_variants = []
        for mutated in variants:
            if mutated == original:
                raise AssertionError(f"{kind} mutation did not change the payload")
            detected = {violation.kind for violation in audit_payload(entry, mutated)}
            detected_variants.append(kind in detected)
            if kind not in detected:
                raise AssertionError(f"{kind} mutation was not detected; got {sorted(detected)}")
        results[kind] = all(detected_variants)

    catalog_mutation = copy.deepcopy(entry)
    catalog_mutation["prompt_directives"][0]["text"] = "Describe the desired visual style."
    catalog_detected = {violation.kind for violation in audit_catalog_entry(catalog_mutation)}
    if "structure" not in catalog_detected:
        raise AssertionError(
            f"authoring-mode catalog mutation was not detected; got {sorted(catalog_detected)}"
        )
    results["structure"] = results["structure"] and "structure" in catalog_detected

    sample_entries = [
        candidate
        for candidate in catalog["entries"]
        if candidate["reference_text_mode"] == "sample_copy"
    ]
    if sample_entries:
        sample_entry = sample_entries[0]
        sample_payload = payloads[sample_entry["id"]]
        exact_copy = json.dumps(sample_entry["reference_exact_text"][0], ensure_ascii=False)
        sample_mutation = sample_payload.replace(exact_copy, "", 1)
        sample_detected = {
            violation.kind for violation in audit_payload(sample_entry, sample_mutation)
        }
        if "completeness" not in sample_detected:
            raise AssertionError(
                f"reference-copy completeness mutation was not detected; got {sorted(sample_detected)}"
            )
        results["completeness"] = (
            results["completeness"] and "completeness" in sample_detected
        )

    deterministic_mutation = original + "\n- Determinism mutation: changed second render"
    if deterministic_mutation == original:
        raise AssertionError("determinism mutation did not change the payload")
    results["determinism"] = any(
        violation.kind == "determinism"
        for violation in audit_determinism(original, deterministic_mutation)
    )
    if not results["determinism"]:
        raise AssertionError("determinism mutation was not detected")
    return results


def render_report(catalog: dict[str, Any], mutation_results: dict[str, bool]) -> str:
    entries = catalog["entries"]
    directive_count = sum(len(entry["prompt_directives"]) for entry in entries)
    reference_modes = {
        mode: sum(1 for entry in entries if entry["reference_text_mode"] == mode)
        for mode in ("sample_copy", "blank_zone", "none")
    }
    lines = [
        "# Prompt Audit Report",
        "",
        "## 결론",
        "",
        f"- Catalog entries: **{len(entries)}**",
        f"- Catalog directives: **{directive_count}**",
        f"- Catalog reference model payloads: **{len(entries)}**",
        f"- Runtime matrix contract: **{len(entries)} × 2 languages = {len(entries) * 2} cases**",
        f"- Reference text contracts: **{reference_modes['sample_copy']} sample-copy / {reference_modes['blank_zone']} intentional blank-zone / {reference_modes['none']} no-copy**",
        f"- Concrete catalog-owned test cases: **{len(json.loads(SCENARIOS_PATH.read_text(encoding='utf-8'))['scenarios'])}**",
        f"- Validator mutation proofs: **{sum(mutation_results.values())}/{len(mutation_results)} detected**",
        "",
        "모든 카테고리는 구조화 렌더러를 사용하며 지정된 조건을 자르지 않는다.",
        "",
        "## 검수 표면",
        "",
        "| Surface | Source of truth | Leakage boundary | Completeness contract | Structure boundary |",
        "|---|---|---|---|---|",
        "| Structured image compiler | catalog + typed request | no ID/path/hash/directive key/intent | every selected directive | semantic slot labels only |",
        "| Codex image adapter | compiled prompt + output parameters | JSON data boundary | one prompt + all parameters | fixed execution contract owns headings |",
        "| Catalog reference generator | catalog directives | metadata outside payload | every directive exactly once | fenced payload only |",
        "",
        "## Mutation proof",
        "",
        "| Defect class | Deliberate mutation | Detected |",
        "|---|---|---:|",
    ]
    descriptions = {
        "leakage": "inject entry.source_path",
        "completeness": "remove/duplicate a directive and remove catalog-owned audit copy",
        "language": "inject untranslated Hangul into English payload",
        "structure": "append an empty label and inject an authoring-mode directive",
        "determinism": "change the second render",
    }
    for kind in ("leakage", "completeness", "language", "structure", "determinism"):
        lines.append(f"| {kind} | {descriptions[kind]} | {'PASS' if mutation_results[kind] else 'FAIL'} |")
    lines.extend(
        [
            "",
            "## Entry-by-entry projection matrix",
            "",
            "| ID | Tier 2 purpose | Directives | Reference text | Structured | Generated payload |",
            "|---|---|---:|---|---|---|",
        ]
    )
    for entry in entries:
        lines.append(
            f"| {entry['id']} | {entry['tier_2']} | {len(entry['prompt_directives'])} | {entry['reference_text_mode']} | all directives | audited |"
        )
    lines.extend(
        [
            "",
            "## 증거",
            "",
            "- Runtime exhaustive test: `crates/promptgen-core/src/image/render.rs::every_catalog_entry_compiles_to_a_clean_prompt`",
            "- Profile validation: `crates/promptgen-core/src/image/validate.rs::validate_render_profile`",
            "- Catalog authoring-source and payload audit: `scripts/audit-prompts.py`",
            "- Deterministic payload generator: `scripts/generate-catalog-image-prompts.py`",
            "- Synchronized renewal gate: `scripts/renew-image-catalog.py --check`",
            "",
        ]
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true", help="fail when generated prompts or report are stale")
    parser.add_argument("--write", action="store_true", help="write the deterministic audit report")
    args = parser.parse_args()

    catalog = json.loads(CATALOG_PATH.read_text(encoding="utf-8"))
    generator = load_generator()
    scenarios = generator.load_scenarios()
    generated_once = generator.render(catalog, scenarios)
    generated_twice = generator.render(catalog, scenarios)
    determinism_violations = audit_determinism(generated_once, generated_twice)
    if determinism_violations:
        for violation in determinism_violations:
            print(f"[FAIL] {violation.kind}: {violation.detail}", file=sys.stderr)
        return 1

    checked_document = PROMPTS_PATH.read_text(encoding="utf-8") if PROMPTS_PATH.is_file() else ""
    if checked_document != generated_once:
        print("[FAIL] catalog model payload document is stale", file=sys.stderr)
        return 1

    violations, payloads = audit_document(catalog, scenarios, checked_document)
    if violations:
        for violation in violations:
            print(
                f"[FAIL] {violation.kind} {violation.entry_id}: {violation.detail}",
                file=sys.stderr,
            )
        return 1

    mutation_results = prove_mutations(catalog, payloads)
    report = render_report(catalog, mutation_results)
    if args.write:
        REPORT_PATH.write_text(report, encoding="utf-8")
    if args.check and (not REPORT_PATH.is_file() or REPORT_PATH.read_text(encoding="utf-8") != report):
        print("[FAIL] prompt audit report is stale", file=sys.stderr)
        return 1

    print(
        "[OK] prompt audit: "
        f"entries={len(catalog['entries'])} "
        f"directives={sum(len(entry['prompt_directives']) for entry in catalog['entries'])} "
        f"payloads={len(payloads)} mutations={sum(mutation_results.values())}/{len(mutation_results)}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
