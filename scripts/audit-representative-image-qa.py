#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CATALOG = json.loads((ROOT / "catalog/image_catalog.json").read_text(encoding="utf-8"))
MANIFEST = json.loads(
    (ROOT / "catalog/CHATGPT_IMAGE_CATALOG_MANIFEST.json").read_text(encoding="utf-8")
)
QA = json.loads(
    (ROOT / "catalog/REPRESENTATIVE_IMAGE_QA.json").read_text(encoding="utf-8")
)


def fail(message: str) -> None:
    raise SystemExit(f"[FAIL] representative image QA: {message}")


if QA.get("format") != "promptgen-representative-image-qa/1":
    fail("unsupported format")

catalog_ids = [entry["id"] for entry in CATALOG["entries"]]
manifest_by_path = {image["asset_path"]: image for image in MANIFEST["images"]}
cases = QA.get("cases")
if not isinstance(cases, list):
    fail("cases must be an array")
if [case.get("id") for case in cases] != catalog_ids:
    fail("case order or IDs do not match the runtime catalog")

for entry, case in zip(CATALOG["entries"], cases):
    if case.get("status") != "PASS":
        fail(f"{entry['id']} is not PASS")
    manifest = manifest_by_path.get(entry["asset_path"])
    if manifest is None:
        fail(f"{entry['id']} has no manifest record")
    asset = ROOT / "catalog" / entry["asset_path"]
    actual_sha = hashlib.sha256(asset.read_bytes()).hexdigest()
    if actual_sha != manifest.get("sha256") or actual_sha != case.get("asset_sha256"):
        fail(f"{entry['id']} review is not bound to the current asset bytes")
    criteria = case.get("criteria")
    if not isinstance(criteria, list) or not criteria:
        fail(f"{entry['id']} has no criteria")
    criterion_ids: set[str] = set()
    for criterion in criteria:
        criterion_id = criterion.get("id")
        if not isinstance(criterion_id, str) or not criterion_id or criterion_id in criterion_ids:
            fail(f"{entry['id']} has a missing or duplicate criterion ID")
        criterion_ids.add(criterion_id)
        if criterion.get("status") != "PASS":
            fail(f"{entry['id']} criterion {criterion_id} is not PASS")
        for field in ("expected", "observed"):
            if not isinstance(criterion.get(field), str) or not criterion[field].strip():
                fail(f"{entry['id']} criterion {criterion_id} has no {field}")

print(
    f"[OK] representative image QA: cases={len(cases)} "
    f"criteria={sum(len(case['criteria']) for case in cases)} manifest-bound"
)
