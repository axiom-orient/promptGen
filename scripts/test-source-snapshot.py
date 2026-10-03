#!/usr/bin/env python3
from __future__ import annotations

import tempfile
from pathlib import Path

from source_snapshot import (
    ensure_output_allowed,
    make_snapshot,
    record_success_verification,
    verify_verification_identity,
)

with tempfile.TemporaryDirectory(prefix="promptgen-source-snapshot-") as temp_dir:
    root = Path(temp_dir)
    (root / "REVISION").write_text("test-revision\n", encoding="utf-8")
    (root / "src").mkdir()
    (root / "src/main.rs").write_text("fn main() {}\n", encoding="utf-8")
    (root / "evals/dogfood/current").mkdir(parents=True)
    (root / "evals/dogfood/current/run.json").write_text("{}\n", encoding="utf-8")
    (root / "COMPLETION_REPORT.md").write_text("pending\n", encoding="utf-8")
    (root / "SOURCE_MANIFEST.sha256").write_text("retired\n", encoding="utf-8")
    (root / "SOURCE_SNAPSHOT.json").write_text("retired\n", encoding="utf-8")
    first = make_snapshot(root)
    assert all(item["path"] not in {"SOURCE_MANIFEST.sha256", "SOURCE_SNAPSHOT.json"} for item in first["files"])
    for retired_name in ("SOURCE_MANIFEST.sha256", "SOURCE_SNAPSHOT.json"):
        try:
            ensure_output_allowed(root / retired_name)
        except ValueError as error:
            assert retired_name in str(error)
        else:
            raise AssertionError(f"{retired_name} must not be generated")
    (root / "evals/dogfood/current/run.json").write_text('{"changed":true}\n', encoding="utf-8")
    (root / "COMPLETION_REPORT.md").write_text("changed\n", encoding="utf-8")
    second = make_snapshot(root)
    assert first["tree_sha256"] == second["tree_sha256"]
    (root / "src/main.rs").write_text("fn main() { println!(\"x\"); }\n", encoding="utf-8")
    third = make_snapshot(root)
    assert third["tree_sha256"] != second["tree_sha256"]

    (root / "VERIFICATION.json").write_text(
        '{"revision":"test-revision","source_tree_sha256":"stale"}\n',
        encoding="utf-8",
    )
    try:
        verify_verification_identity(root, third)
    except ValueError as error:
        assert "source_tree_sha256" in str(error)
    else:
        raise AssertionError("stale verification identity must fail closed")

    (root / "VERIFICATION.json").write_text(
        "{\n"
        '  "revision": "test-revision",\n'
        f'  "source_tree_sha256": "{third["tree_sha256"]}"\n'
        "}\n",
        encoding="utf-8",
    )
    assert verify_verification_identity(root, third)["revision"] == "test-revision"

    receipt = record_success_verification(root, third)
    assert receipt["status"] == "CURRENT_LOCAL_GATE_COMPLETED"
    assert receipt["source_tree_sha256"] == third["tree_sha256"]
    assert verify_verification_identity(root, third)["status"] == "CURRENT_LOCAL_GATE_COMPLETED"

    (root / "VERIFICATION.json").write_text(
        "{\n"
        '  "revision": "wrong-revision",\n'
        f'  "source_tree_sha256": "{third["tree_sha256"]}"\n'
        "}\n",
        encoding="utf-8",
    )
    try:
        verify_verification_identity(root, third)
    except ValueError as error:
        assert "revision" in str(error)
    else:
        raise AssertionError("revision mismatch must fail closed")

with tempfile.TemporaryDirectory(prefix="promptgen-source-symlink-") as temp_dir:
    root = Path(temp_dir)
    (root / "REVISION").write_text("test-revision\n", encoding="utf-8")
    target = root / "target.txt"
    target.write_text("target\n", encoding="utf-8")
    link = root / "link.txt"
    try:
        link.symlink_to(target)
    except OSError:
        pass
    else:
        try:
            make_snapshot(root)
        except ValueError as error:
            assert "rejects symlink" in str(error)
        else:
            raise AssertionError("source snapshot must reject symlinks")

print("source-snapshot: PASS")
