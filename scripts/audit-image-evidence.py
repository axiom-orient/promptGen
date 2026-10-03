#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import json
import struct
from pathlib import Path, PurePosixPath

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "evals/dogfood/historical/1.4.0/image/image-evidence.json"
ANALYSIS = ROOT / "evals/dogfood/historical/1.4.0/image/IMAGE_RESULT_ANALYSIS.md"
CURRENT_POLICY = ROOT / "evals/dogfood/current/IMAGE_EVIDENCE_POLICY.md"
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def fail(message: str) -> None:
    raise SystemExit(f"[FAIL] {message}")


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def png_size(path: Path) -> tuple[int, int]:
    with path.open("rb") as handle:
        header = handle.read(24)
    if len(header) != 24 or header[:8] != PNG_SIGNATURE or header[12:16] != b"IHDR":
        fail(f"invalid PNG header: {path.relative_to(ROOT)}")
    return struct.unpack(">II", header[16:24])


data = json.loads(EVIDENCE.read_text(encoding="utf-8"))
if data.get("format") != "promptgen-historical-image-evidence/1":
    fail("historical image evidence format drifted")
if data.get("candidate_revision") != "promptgen-1.4.0-20260727.1":
    fail("historical image evidence revision drifted")
if data.get("transport_observed") is not False:
    fail("historical image transport must remain explicitly unobserved")
records = data.get("records")
if not isinstance(records, list) or len(records) != 2:
    fail("expected exactly two historical control images")
for record in records:
    relative = record.get("path")
    if not isinstance(relative, str):
        fail("image evidence path must be a string")
    pure = PurePosixPath(relative)
    if pure.is_absolute() or ".." in pure.parts or not relative.startswith("evals/dogfood/historical/1.4.0/"):
        fail(f"unsafe or non-historical image path: {relative!r}")
    path = ROOT / relative
    if not path.is_file():
        fail(f"missing image evidence: {relative}")
    if path.stat().st_size != record.get("bytes"):
        fail(f"image byte mismatch: {relative}")
    if sha256(path) != record.get("sha256"):
        fail(f"image SHA-256 mismatch: {relative}")
    if png_size(path) != (record.get("width"), record.get("height")):
        fail(f"image dimension mismatch: {relative}")
analysis = ANALYSIS.read_text(encoding="utf-8")
for token in [
    "STATUS: FAIL",
    "CAUSE_CLASS: PROMPT_TRANSPORT_CONTEXT_CONTAMINATION",
    "submitted-prompt fidelity | 0/4 | 0/4",
    "historical transport-failure evidence",
]:
    if token not in analysis:
        fail(f"historical image analysis is missing: {token}")
policy = CURRENT_POLICY.read_text(encoding="utf-8")
for token in ["does not inherit image fidelity scores", "NOT_RUN_EXTERNAL"]:
    if token not in policy:
        fail(f"current image evidence policy is missing: {token}")
print(f"[OK] historical image evidence: files={len(records)} transport_observed=false fidelity=0/4")
