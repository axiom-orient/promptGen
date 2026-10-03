#!/usr/bin/env python3
"""Check the maintained image-only responsibility boundaries."""

from __future__ import annotations

import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
EXPECTED_PRIMARY = ["C1", "C4", "C5", "C6", "C10", "C11"]
PROFILE_TOKENS = [
    "standard",
    "travel_journal",
    "logo_identity",
    "app_icon",
    "app_web_ui",
    "information_design",
    "character_pose",
    "poomsae_pose",
]


def fail(message: str) -> None:
    raise SystemExit(f"[FAIL] architecture audit: {message}")


def read(relative: str) -> str:
    path = ROOT / relative
    if not path.is_file():
        fail(f"missing required file {relative}")
    return path.read_text(encoding="utf-8")


def require(text: str, marker: str, surface: str) -> None:
    if marker not in text:
        fail(f"{surface} is missing {marker!r}")


catalog = json.loads(read("catalog/image_catalog.json"))
entries = catalog.get("entries")
if not isinstance(entries, list):
    fail("catalog entries must be an array")
primary = [entry["id"] for entry in entries if entry.get("tier_1") == "결과물"]
if primary != EXPECTED_PRIMARY:
    fail(f"primary catalog outcomes drifted: expected {EXPECTED_PRIMARY!r}, got {primary!r}")
if any(not isinstance(entry.get("profiles"), list) for entry in entries):
    fail("every catalog entry must advertise a profile list")

core_model = read("crates/promptgen-core/src/image/model.rs")
for token in PROFILE_TOKENS:
    require(core_model, f'"{token}"', "image model profile schema")
require(core_model, "pub enum ImageProfile", "image model")
require(core_model, 'take_optional(&mut fields, "profile")', "image decoder")
require(core_model, '("profile", JsonValue::from(self.profile.as_str()))', "image serializer")

core_validation = read("crates/promptgen-core/src/image/validate.rs")
for marker in [
    "fn validate_image_profile(",
    "IMG_PROFILE_CATEGORY",
    "IMG_PROFILE_CATALOG",
    "IMG_LOGO_PROFILE_FORMAT",
    "IMG_APP_ICON_DIMENSIONS",
    "IMG_APP_ICON_PLANES",
    "IMG_APP_ICON_FORBIDDEN_CONTENT",
    "IMG_UI_PROFILE_SCREEN",
    "IMG_POOMSAE_PROFILE_INTENT",
]:
    require(core_validation, marker, "image profile validation")

core_render = read("crates/promptgen-core/src/image/render.rs")
for marker in [
    "fn render_category_profile_contract(",
    "opaque PNG concept and brand-mark direction only",
    "APP ICON MASTER CONCEPT CONTRACT",
    "text-directed poomsae concept",
]:
    require(core_render, marker, "image profile rendering")

schema = read("schemas/image.schema.json")
for token in PROFILE_TOKENS:
    require(schema, f'"{token}"', "checked-in image schema")

interview = read("crates/promptgen-core/src/interview/image.rs")
for marker in [
    "infer_profile_from_brief(",
    "profile_question(",
    'answers.insert("image.profile"',
    "profile_is_compatible(",
]:
    require(interview, marker, "guided image interview")
inference = read("crates/promptgen-core/src/interview/image/inference.rs")
require(inference, "pub(super) fn infer_profile(", "profile inference")
questions = read("crates/promptgen-core/src/interview/image/questions.rs")
require(questions, '"image.profile"', "profile interview question")

studio = read("crates/promptgen-web/src/assets/app.js")
for marker in [
    "const IMAGE_PROFILE_OPTIONS =",
    'value: "app_icon"',
    '"image.profile"',
    "routeProfileSelect",
]:
    require(studio, marker, "Studio profile surface")

cli = read("crates/promptgen-cli/src/main.rs")
require(cli, "image", "image CLI")
mcp = read("crates/promptgen-mcp/src/lib.rs")
require(mcp, "image", "image MCP surface")

print(
    "[OK] image-only architecture: six primary outcomes, typed category profiles, "
    "shared core validation/rendering, guided interview, Studio, CLI, and MCP boundaries"
)
