#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
BIN=${1:-${PROMPTGEN_BIN:-$ROOT/target/release/promptgen}}
[[ -x "$BIN" ]] || { echo "[FAIL] promptgen binary is not executable: $BIN" >&2; exit 1; }
TMP=$(mktemp -d "${TMPDIR:-/tmp}/promptgen-image-smoke.XXXXXX")
trap 'rm -rf "$TMP"' EXIT
VERSION=$(sed -n 's/^version = "\(.*\)"$/\1/p' "$ROOT/Cargo.toml" | head -n 1)
[[ "$($BIN version)" == "promptgen $VERSION" ]]

for example in image-photo-lut image-typography-poster image-logo-identity image-app-icon image-app-web-ui image-character-poomsae image-pose-transfer; do
  "$BIN" image --input "$ROOT/examples/$example.json" --mode prompt-only --format json >"$TMP/$example.json"
  grep -q '"kind": "image"' "$TMP/$example.json"
  grep -q '"status": "valid' "$TMP/$example.json"
done
grep -q 'logo_identity' "$TMP/image-logo-identity.json"
grep -q 'app_icon' "$TMP/image-app-icon.json"
grep -q '앱 아이콘 마스터 콘셉트 계약' "$TMP/image-app-icon.json"
grep -q 'app_web_ui' "$TMP/image-app-web-ui.json"
grep -q 'poomsae_pose' "$TMP/image-character-poomsae.json"
[[ "$($BIN catalog --format text | wc -l | tr -d ' ')" == 6 ]]
"$BIN" schema image >"$TMP/image.schema.json"
"$BIN" schema interview >"$TMP/interview.schema.json"
"$BIN" schema compilation >"$TMP/compilation.schema.json"
grep -q 'logo_identity' "$TMP/image.schema.json"
grep -q 'app_icon' "$TMP/image.schema.json"
grep -q '"image"' "$TMP/interview.schema.json"
grep -q '"image"' "$TMP/compilation.schema.json"
"$BIN" lut-presets >"$TMP/lut.json"
grep -q 'monochrome_high_contrast' "$TMP/lut.json"
"$BIN" image --input "$ROOT/examples/image-photo-lut.json" --format json >"$TMP/a.json"
"$BIN" image --input "$ROOT/examples/image-photo-lut.json" --format json >"$TMP/b.json"
cmp "$TMP/a.json" "$TMP/b.json"
echo "[OK] image CLI/catalog/schema/interview smoke"
