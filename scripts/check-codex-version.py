#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import subprocess
from pathlib import Path

VERSION_RE = re.compile(r"(?<!\d)(\d+)\.(\d+)\.(\d+)(?!\d)")


def parse_version(text: str) -> tuple[int, int, int]:
    match = VERSION_RE.search(text)
    if not match:
        raise ValueError(f"no semantic version found in {text!r}")
    return tuple(int(value) for value in match.groups())


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("binary")
    parser.add_argument("--minimum", default="0.144.0")
    args = parser.parse_args()
    try:
        minimum = parse_version(args.minimum)
    except ValueError as error:
        raise SystemExit(f"[FAIL] invalid minimum version: {error}") from error
    try:
        result = subprocess.run([args.binary, "--version"], text=True, capture_output=True, timeout=15)
    except (OSError, subprocess.TimeoutExpired) as error:
        raise SystemExit(f"[UNAVAILABLE] failed to run {args.binary!r} --version: {error}") from error
    if result.returncode != 0:
        raise SystemExit(
            f"[FAIL] {args.binary!r} --version exited {result.returncode}: stdout={result.stdout!r} stderr={result.stderr!r}"
        )
    output = (result.stdout + "\n" + result.stderr).strip()
    try:
        actual = parse_version(output)
    except ValueError as error:
        raise SystemExit(f"[FAIL] cannot parse Codex CLI version: {error}") from error
    if actual < minimum:
        raise SystemExit(
            f"[FAIL] Codex CLI {actual[0]}.{actual[1]}.{actual[2]} is below required {minimum[0]}.{minimum[1]}.{minimum[2]}"
        )
    print(f"[OK] Codex CLI version {actual[0]}.{actual[1]}.{actual[2]} >= {minimum[0]}.{minimum[1]}.{minimum[2]}")


if __name__ == "__main__":
    main()
