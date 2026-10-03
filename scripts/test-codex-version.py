#!/usr/bin/env python3
from __future__ import annotations

import importlib.util
import subprocess
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "scripts/check-codex-version.py"
spec = importlib.util.spec_from_file_location("check_codex_version", SCRIPT)
module = importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(module)
assert module.parse_version("codex-cli 0.144.0") == (0, 144, 0)
assert module.parse_version("codex 1.2.3-beta") == (1, 2, 3)
try:
    module.parse_version("codex unknown")
except ValueError:
    pass
else:
    raise AssertionError("unparseable version must fail")

with tempfile.TemporaryDirectory(prefix="promptgen-codex-version-") as temp_dir:
    root = Path(temp_dir)
    old = root / "old"
    current = root / "current"
    malformed = root / "malformed"
    for path, output in [(old, "codex-cli 0.143.9"), (current, "codex-cli 0.144.0"), (malformed, "codex-cli unknown")]:
        path.write_text(f"#!/bin/sh\nprintf '%s\\n' '{output}'\n", encoding="utf-8")
        path.chmod(0o755)
    assert subprocess.run(["python3", str(SCRIPT), str(current)], capture_output=True).returncode == 0
    assert subprocess.run(["python3", str(SCRIPT), str(old)], capture_output=True).returncode != 0
    assert subprocess.run(["python3", str(SCRIPT), str(malformed)], capture_output=True).returncode != 0
print("codex-version-check: PASS")
