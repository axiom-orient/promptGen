#!/usr/bin/env python3
from __future__ import annotations

import re
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUST_ROOTS = [ROOT / "crates"]
OPEN_TO_CLOSE = {"(": ")", "[": "]", "{": "}"}
CLOSE_TO_OPEN = {value: key for key, value in OPEN_TO_CLOSE.items()}
PATH_MODULE = re.compile(r'#\[path\s*=\s*"([^"]+)"\]\s*\n\s*mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*;', re.M)


def fail(message: str) -> None:
    raise SystemExit(f"[FAIL] {message}")


def line_column(text: str, offset: int) -> tuple[int, int]:
    line = text.count("\n", 0, offset) + 1
    previous = text.rfind("\n", 0, offset)
    return line, offset - previous


def char_literal_end(text: str, start: int) -> int | None:
    index = start + 1
    if index >= len(text):
        return None
    if text[index] == "\\":
        index += 2
        if index <= len(text) and text[index - 1] == "u" and index < len(text) and text[index] == "{":
            closing = text.find("}", index + 1)
            if closing == -1:
                return None
            index = closing + 1
    else:
        if text[index] in "\n\r'":
            return None
        index += 1
    return index + 1 if index < len(text) and text[index] == "'" else None


def raw_string_start(text: str, start: int) -> tuple[int, int] | None:
    index = start
    if text.startswith("br", index) or text.startswith("cr", index):
        index += 1
    if index >= len(text) or text[index] != "r":
        return None
    index += 1
    hashes = 0
    while index < len(text) and text[index] == "#":
        hashes += 1
        index += 1
    if index >= len(text) or text[index] != '"':
        return None
    return index + 1, hashes


def audit_file(path: Path) -> tuple[int, int]:
    text = path.read_text(encoding="utf-8")
    relative = path.relative_to(ROOT).as_posix()
    if "\x00" in text:
        fail(f"{relative}: contains NUL")
    for marker in ["<<<<<<<", "=======", ">>>>>>>"]:
        if marker in text:
            fail(f"{relative}: contains merge marker {marker}")

    stack: list[tuple[str, int]] = []
    block_depth = 0
    index = 0
    while index < len(text):
        if block_depth:
            if text.startswith("/*", index):
                block_depth += 1
                index += 2
            elif text.startswith("*/", index):
                block_depth -= 1
                index += 2
            else:
                index += 1
            continue
        if text.startswith("//", index):
            newline = text.find("\n", index + 2)
            index = len(text) if newline == -1 else newline + 1
            continue
        if text.startswith("/*", index):
            block_depth = 1
            index += 2
            continue

        raw = raw_string_start(text, index)
        if raw is not None:
            content_start, hashes = raw
            terminator = '"' + ("#" * hashes)
            end = text.find(terminator, content_start)
            if end == -1:
                line, column = line_column(text, index)
                fail(f"{relative}:{line}:{column}: unterminated raw string")
            index = end + len(terminator)
            continue

        if text[index] in {'"'} or (
            text[index] in {"b", "c"} and index + 1 < len(text) and text[index + 1] == '"'
        ):
            if text[index] != '"':
                index += 1
            index += 1
            while index < len(text):
                if text[index] == "\\":
                    index += 2
                elif text[index] == '"':
                    index += 1
                    break
                else:
                    index += 1
            else:
                line, column = line_column(text, index - 1)
                fail(f"{relative}:{line}:{column}: unterminated string")
            continue

        if text[index] == "'":
            end = char_literal_end(text, index)
            if end is not None:
                index = end
                continue

        character = text[index]
        if character in OPEN_TO_CLOSE:
            stack.append((character, index))
        elif character in CLOSE_TO_OPEN:
            if not stack or stack[-1][0] != CLOSE_TO_OPEN[character]:
                line, column = line_column(text, index)
                fail(f"{relative}:{line}:{column}: unmatched {character}")
            stack.pop()
        index += 1

    if block_depth:
        fail(f"{relative}: unterminated block comment")
    if stack:
        character, offset = stack[-1]
        line, column = line_column(text, offset)
        fail(f"{relative}:{line}:{column}: unclosed {character}")

    path_modules = 0
    for match in PATH_MODULE.finditer(text):
        module_path = path.parent / match.group(1)
        if not module_path.is_file():
            line, column = line_column(text, match.start())
            fail(
                f"{relative}:{line}:{column}: path module {match.group(2)!r} is missing: "
                f"{module_path.relative_to(ROOT)}"
            )
        path_modules += 1
    return text.count("#[test]"), path_modules


def main() -> None:
    files = sorted(
        path
        for root in RUST_ROOTS
        for path in root.rglob("*.rs")
        if "target" not in path.parts
    )
    if not files:
        fail("no Rust source files found")
    tests = 0
    path_modules = 0
    for path in files:
        file_tests, file_modules = audit_file(path)
        tests += file_tests
        path_modules += file_modules
    print(
        f"[OK] Rust lexical/module structure: files={len(files)} "
        f"test_markers={tests} path_modules={path_modules}"
    )


if __name__ == "__main__":
    main()
