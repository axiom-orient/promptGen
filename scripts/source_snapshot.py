#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path

# Build output, caches, and per-developer editor or agent state are not maintained
# source. Including them made the tree hash depend on whichever tool last touched
# the checkout, so an unrelated local settings file failed the release gate.
EXCLUDED_PARTS = {
    ".git",
    "target",
    "dist",
    "deliverables",
    "node_modules",
    "__pycache__",
    ".pytest_cache",
    "__MACOSX",
    ".claude",
    ".vscode",
    ".idea",
    "var",
}
RETIRED_OUTPUT_NAMES = {
    "SOURCE_MANIFEST.sha256",
    "SOURCE_SNAPSHOT.json",
}
EXCLUDED_NAMES = {
    ".DS_Store",
    "VERIFICATION.json",
    "COMPLETION_REPORT.md",
} | RETIRED_OUTPUT_NAMES
EXCLUDED_PREFIXES = (
    "evals/dogfood/current/",
)


def source_files(root: Path) -> list[Path]:
    files: list[Path] = []
    for path in root.rglob("*"):
        if path.is_symlink():
            raise ValueError(f"source snapshot rejects symlink: {path.relative_to(root)}")
        if not path.is_file():
            continue
        relative = path.relative_to(root)
        relative_text = relative.as_posix()
        if any(part in EXCLUDED_PARTS for part in relative.parts):
            continue
        if path.name in EXCLUDED_NAMES or path.suffix in {".pyc", ".zip"}:
            continue
        if relative_text.startswith(EXCLUDED_PREFIXES):
            continue
        files.append(path)
    return sorted(files, key=lambda path: path.relative_to(root).as_posix())


def make_snapshot(root: Path) -> dict:
    records = []
    aggregate = hashlib.sha256()
    total_bytes = 0
    for path in source_files(root):
        relative = path.relative_to(root).as_posix()
        data = path.read_bytes()
        digest = hashlib.sha256(data).hexdigest()
        total_bytes += len(data)
        aggregate.update(relative.encode("utf-8"))
        aggregate.update(b"\0")
        aggregate.update(str(len(data)).encode("ascii"))
        aggregate.update(b"\0")
        aggregate.update(digest.encode("ascii"))
        aggregate.update(b"\n")
        records.append({"path": relative, "bytes": len(data), "sha256": digest})
    revision = (root / "REVISION").read_text(encoding="utf-8").strip()
    return {
        "format": "promptgen-source-snapshot/1",
        "revision": revision,
        "scope": {
            "description": "product source and maintained documentation; excludes generated current-run evidence, completion/package manifests, archives, caches, and build outputs",
            "excluded_prefixes": list(EXCLUDED_PREFIXES),
            "excluded_names": sorted(EXCLUDED_NAMES),
            "excluded_parts": sorted(EXCLUDED_PARTS),
        },
        "file_count": len(records),
        "total_bytes": total_bytes,
        "tree_sha256": aggregate.hexdigest(),
        "files": records,
    }


def verify_verification_identity(root: Path, snapshot: dict) -> dict:
    """Require the checked-in verification receipt to attest this exact source."""
    verification_path = root / "VERIFICATION.json"
    try:
        verification = json.loads(verification_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"cannot read VERIFICATION.json: {error}") from error
    if not isinstance(verification, dict):
        raise ValueError("VERIFICATION.json must contain an object")

    expected_revision = snapshot.get("revision")
    verified_revision = verification.get("revision")
    if verified_revision != expected_revision:
        raise ValueError(
            "VERIFICATION.json revision does not match the source snapshot: "
            f"{verified_revision!r} != {expected_revision!r}"
        )

    expected_sha256 = snapshot.get("tree_sha256")
    verified_sha256 = verification.get("source_tree_sha256")
    if verified_sha256 != expected_sha256:
        raise ValueError(
            "VERIFICATION.json source_tree_sha256 does not match the current maintained source: "
            f"{verified_sha256!r} != {expected_sha256!r}"
        )
    return verification



def record_success_verification(root: Path, snapshot: dict) -> dict:
    """Atomically record a successful full local gate for this exact source tree."""
    receipt = {
        "format": "promptgen-verification/4",
        "scope": "complete local gate: check/test/fmt/clippy/doc/build/audits/CLI/web smoke",
        "status": "CURRENT_LOCAL_GATE_COMPLETED",
        "revision": snapshot["revision"],
        "source_tree_sha256": snapshot["tree_sha256"],
        "checks": [
            {
                "command": "bash scripts/verify.sh",
                "exitCode": 0,
                "scope": "check/test/fmt/clippy/doc/build/audits/CLI/web smoke",
            }
        ],
    }
    destination = root / "VERIFICATION.json"
    temporary = root / ".VERIFICATION.json.tmp"
    temporary.write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    os.replace(temporary, destination)
    return receipt

def ensure_output_allowed(output: Path) -> None:
    if output.name in RETIRED_OUTPUT_NAMES:
        raise ValueError(f"refusing to generate retired root artifact: {output.name}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--root", default=str(Path(__file__).resolve().parents[1]))
    parser.add_argument("--output")
    parser.add_argument(
        "--verify-verification",
        action="store_true",
        help="verify VERIFICATION.json against the selected root snapshot",
    )
    parser.add_argument(
        "--record-verification",
        action="store_true",
        help="record a successful full verification receipt for the selected root snapshot",
    )
    args = parser.parse_args()
    root = Path(args.root).resolve()
    snapshot = make_snapshot(root)
    if args.verify_verification and args.record_verification:
        raise SystemExit("choose either --verify-verification or --record-verification")
    if args.record_verification:
        receipt = record_success_verification(root, snapshot)
        print(
            "verification-recorded: PASS "
            f"revision={receipt['revision']} tree_sha256={receipt['source_tree_sha256']}"
        )
        return
    if args.verify_verification:
        verify_verification_identity(root, snapshot)
        print(
            "verification-identity: PASS "
            f"revision={snapshot['revision']} tree_sha256={snapshot['tree_sha256']}"
        )
        return
    text = json.dumps(snapshot, ensure_ascii=False, indent=2) + "\n"
    if args.output:
        output = Path(args.output).resolve()
        ensure_output_allowed(output)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(text, encoding="utf-8")
    else:
        print(text, end="")


if __name__ == "__main__":
    main()
