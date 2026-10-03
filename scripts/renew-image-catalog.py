#!/usr/bin/env python3
"""Synchronize catalog-derived prompt documents from the six-outcome catalog."""

from __future__ import annotations

import argparse
import importlib.util
import json
import os
import pathlib
import sys
import tempfile
from typing import Any


ROOT = pathlib.Path(__file__).resolve().parent.parent
CATALOG = ROOT / "catalog" / "image_catalog.json"
GENERATOR = ROOT / "scripts" / "generate-catalog-image-prompts.py"
AUDITOR = ROOT / "scripts" / "audit-prompts.py"
EXPECTED_IDS = ["C1", "C4", "C5", "C6", "C10", "C11"]


def load_script(name: str, path: pathlib.Path):
    spec = importlib.util.spec_from_file_location(name, path)
    if spec is None or spec.loader is None:
        raise RuntimeError(f"cannot import {path}")
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


def validate_catalog(data: dict[str, Any]) -> None:
    entries = data.get("entries")
    if not isinstance(entries, list):
        raise ValueError("catalog.entries must be an array")
    ids = [entry.get("id") for entry in entries]
    if ids != EXPECTED_IDS:
        raise ValueError(f"expected representative outcomes {EXPECTED_IDS}, found {ids}")
    if data.get("schema_version") != 5:
        raise ValueError("six-outcome catalog requires schema_version=5")
    if any(entry.get("tier_1") != "결과물" for entry in entries):
        raise ValueError("every v5 catalog entry must be a primary outcome")
    if any(len(entry.get("prompt_directives", [])) != 3 for entry in entries):
        raise ValueError("every representative outcome must have exactly three directives")


def render_outputs(data: dict[str, Any]) -> dict[pathlib.Path, str]:
    generator = load_script("promptgen_catalog_prompt_generator", GENERATOR)
    auditor = load_script("promptgen_prompt_auditor", AUDITOR)
    scenarios = generator.load_scenarios()
    unknown = sorted(set(scenarios) - set(EXPECTED_IDS))
    if unknown:
        raise ValueError(f"reference scenarios contain removed IDs: {unknown}")
    prompt_document = generator.render(data, scenarios)
    if prompt_document != generator.render(data, scenarios):
        raise RuntimeError("catalog prompt generator is not deterministic")
    violations, payloads = auditor.audit_document(data, scenarios, prompt_document)
    if violations:
        details = "; ".join(
            f"{item.kind} {item.entry_id}: {item.detail}" for item in violations
        )
        raise RuntimeError(f"catalog prompt audit failed: {details}")
    mutation_results = auditor.prove_mutations(data, payloads)
    if not all(mutation_results.values()):
        raise RuntimeError(f"prompt auditor mutation proof failed: {mutation_results}")
    return {
        generator.OUTPUT: prompt_document,
        auditor.REPORT_PATH: auditor.render_report(data, mutation_results),
    }


def write_atomic(path: pathlib.Path, content: str) -> None:
    descriptor, name = tempfile.mkstemp(
        dir=path.parent, prefix=f".{path.name}.", suffix=".tmp", text=True
    )
    temporary = pathlib.Path(name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="") as handle:
            handle.write(content)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="fail when generated prompt documents are stale",
    )
    args = parser.parse_args()
    data = json.loads(CATALOG.read_text(encoding="utf-8"))
    validate_catalog(data)
    outputs = render_outputs(data)
    stale = [
        path.relative_to(ROOT).as_posix()
        for path, expected in outputs.items()
        if not path.is_file() or path.read_text(encoding="utf-8") != expected
    ]
    if args.check:
        if stale:
            print(f"stale prompt assets: {', '.join(stale)}", file=sys.stderr)
            return 1
    else:
        for path, content in outputs.items():
            write_atomic(path, content)
    directive_count = sum(
        len(entry["prompt_directives"]) for entry in data["entries"]
    )
    print(
        f"[OK] prompt assets synchronized: entries={len(data['entries'])} "
        f"directives={directive_count}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
