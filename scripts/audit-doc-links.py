#!/usr/bin/env python3
from __future__ import annotations

import re
from pathlib import Path
from urllib.parse import unquote

ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r"!?\[[^\]]*\]\(([^)]+)\)")
SKIP_PREFIXES = ("http://", "https://", "mailto:", "#", "sandbox:")


def fail(message: str) -> None:
    raise SystemExit(f"[FAIL] {message}")


checked = 0
for document in sorted(ROOT.rglob("*.md")):
    relative_document = document.relative_to(ROOT)
    if any(part in {"target", "node_modules", "__MACOSX"} for part in relative_document.parts):
        continue
    text = document.read_text(encoding="utf-8")
    for raw in LINK.findall(text):
        target_text = raw.strip().split(maxsplit=1)[0].strip("<>")
        if not target_text or target_text.startswith(SKIP_PREFIXES):
            continue
        target_text = unquote(target_text.split("#", 1)[0])
        if not target_text:
            continue
        target = (document.parent / target_text).resolve()
        try:
            target.relative_to(ROOT)
        except ValueError:
            fail(f"documentation link escapes repository: {relative_document}:{raw}")
        if not target.exists():
            fail(f"broken documentation link: {relative_document} -> {target_text}")
        checked += 1
print(f"[OK] documentation links: {checked}")
