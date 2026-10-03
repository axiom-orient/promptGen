#!/usr/bin/env python3
"""Verify that every catalog PNG exists and that the checked-in image manifest is exact."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import pathlib
import struct
import sys
import tempfile
import zlib
from typing import Any

ROOT = pathlib.Path(__file__).resolve().parent.parent
CATALOG_PATH = ROOT / "catalog" / "image_catalog.json"
MANIFEST_PATH = ROOT / "catalog" / "CHATGPT_IMAGE_CATALOG_MANIFEST.json"
PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def inspect_png(path: pathlib.Path) -> tuple[int, int, int, str]:
    data = path.read_bytes()
    if not data.startswith(PNG_SIGNATURE):
        raise ValueError(f"{path}: invalid PNG signature")
    offset = len(PNG_SIGNATURE)
    width = height = 0
    saw_ihdr = saw_iend = False
    while offset < len(data):
        if offset + 12 > len(data):
            raise ValueError(f"{path}: truncated PNG chunk header")
        length = struct.unpack(">I", data[offset : offset + 4])[0]
        chunk_type = data[offset + 4 : offset + 8]
        chunk_end = offset + 12 + length
        if chunk_end > len(data):
            raise ValueError(f"{path}: truncated PNG chunk {chunk_type!r}")
        payload = data[offset + 8 : offset + 8 + length]
        expected_crc = struct.unpack(">I", data[offset + 8 + length : chunk_end])[0]
        actual_crc = zlib.crc32(chunk_type)
        actual_crc = zlib.crc32(payload, actual_crc) & 0xFFFFFFFF
        if actual_crc != expected_crc:
            raise ValueError(f"{path}: CRC mismatch in {chunk_type.decode('ascii', 'replace')}")
        if chunk_type == b"IHDR":
            if saw_ihdr or offset != len(PNG_SIGNATURE) or length != 13:
                raise ValueError(f"{path}: invalid IHDR placement or size")
            width, height, bit_depth, color_type, compression, filter_method, interlace = struct.unpack(
                ">IIBBBBB", payload
            )
            if width == 0 or height == 0:
                raise ValueError(f"{path}: zero-sized PNG")
            if compression != 0 or filter_method != 0 or interlace not in {0, 1}:
                raise ValueError(f"{path}: unsupported PNG header values")
            if bit_depth != 8 or color_type not in {2, 6}:
                raise ValueError(
                    f"{path}: catalog preview must be 8-bit RGB/RGBA, got bit_depth={bit_depth} color_type={color_type}"
                )
            saw_ihdr = True
        if chunk_type == b"IEND":
            if length != 0:
                raise ValueError(f"{path}: invalid IEND length")
            saw_iend = True
            if chunk_end != len(data):
                raise ValueError(f"{path}: trailing bytes after IEND")
            break
        offset = chunk_end
    if not saw_ihdr or not saw_iend:
        raise ValueError(f"{path}: missing IHDR or IEND")
    return len(data), width, height, hashlib.sha256(data).hexdigest()


def expected_manifest(catalog: dict[str, Any], manifest: dict[str, Any]) -> tuple[dict[str, Any], list[tuple[str, int, int]]]:
    entries = catalog.get("entries")
    images = manifest.get("images")
    if not isinstance(entries, list) or not isinstance(images, list):
        raise ValueError("catalog.entries and manifest.images must be arrays")
    if len(entries) != 6 or len(images) != len(entries):
        raise ValueError(f"catalog/manifest count mismatch: entries={len(entries)} images={len(images)}")

    image_by_path: dict[str, dict[str, Any]] = {}
    for image in images:
        path = image.get("asset_path") or image.get("file")
        if not isinstance(path, str) or path in image_by_path:
            raise ValueError(f"manifest has missing or duplicate asset path {path!r}")
        image_by_path[path] = image

    expected_images: list[dict[str, Any]] = []
    dimensions: list[tuple[str, int, int]] = []
    seen_ids: set[str] = set()
    seen_paths: set[str] = set()
    catalog_root = (ROOT / "catalog").resolve()
    for number, entry in enumerate(entries, 1):
        entry_id = entry.get("id")
        asset_path = entry.get("asset_path")
        if not isinstance(entry_id, str) or not entry_id or entry_id in seen_ids:
            raise ValueError(f"catalog has missing or duplicate id {entry_id!r}")
        if not isinstance(asset_path, str) or asset_path in seen_paths:
            raise ValueError(f"catalog has missing or duplicate asset_path {asset_path!r}")
        seen_ids.add(entry_id)
        seen_paths.add(asset_path)
        pure = pathlib.PurePosixPath(asset_path)
        if pure.is_absolute() or ".." in pure.parts or not asset_path.startswith("assets/") or pure.suffix.lower() != ".png":
            raise ValueError(f"{entry_id}: unsafe catalog asset path {asset_path!r}")
        path = ROOT / "catalog" / pure
        if path.is_symlink() or not path.is_file():
            raise ValueError(f"{entry_id}: missing regular PNG {path}")
        resolved = path.resolve()
        if catalog_root not in resolved.parents:
            raise ValueError(f"{entry_id}: asset escapes catalog root")
        byte_count, width, height, sha256 = inspect_png(path)
        dimensions.append((entry_id, width, height))

        current = image_by_path.get(asset_path)
        if current is None:
            raise ValueError(f"{entry_id}: manifest entry missing for {asset_path}")
        updated = dict(current)
        updated["number"] = number
        updated["title"] = f"{entry_id} — {entry.get('name_ko') or entry.get('name_en') or entry_id}"
        updated["file"] = asset_path
        updated["asset_path"] = asset_path
        updated["bytes"] = byte_count
        updated["sha256"] = sha256
        expected_images.append(updated)

    extra_paths = sorted(set(image_by_path) - seen_paths)
    if extra_paths:
        raise ValueError(f"manifest has unreferenced assets: {extra_paths[:5]}")
    expected = dict(manifest)
    expected["images"] = expected_images
    return expected, dimensions


def write_atomic(path: pathlib.Path, text: str) -> None:
    descriptor, name = tempfile.mkstemp(dir=path.parent, prefix=f".{path.name}.", suffix=".tmp", text=True)
    temporary = pathlib.Path(name)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8", newline="") as handle:
            handle.write(text)
            handle.flush()
            os.fsync(handle.fileno())
        os.replace(temporary, path)
    finally:
        temporary.unlink(missing_ok=True)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--write", action="store_true", help="update only manifest bytes/hashes from verified PNGs")
    args = parser.parse_args()
    catalog = json.loads(CATALOG_PATH.read_text(encoding="utf-8"))
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    expected, dimensions = expected_manifest(catalog, manifest)
    rendered = json.dumps(expected, ensure_ascii=False, indent=2) + "\n"
    current = MANIFEST_PATH.read_text(encoding="utf-8")
    if current != rendered:
        if not args.write:
            print("[FAIL] catalog image manifest is stale", file=sys.stderr)
            return 1
        write_atomic(MANIFEST_PATH, rendered)
    total_bytes = sum(image["bytes"] for image in expected["images"])
    shape_count = len({(width, height) for _, width, height in dimensions})
    print(
        f"[OK] catalog assets: images={len(dimensions)} bytes={total_bytes} "
        f"dimension_shapes={shape_count} manifest={'updated' if current != rendered else 'current'}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
