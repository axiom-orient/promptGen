#!/usr/bin/env bash
# promptGen -> perfectpixel, composed at the shell.
#
# The tools remain independent binaries. promptGen owns typed-request compilation and
# validated PNG publication; perfectpixel owns format derivation from an already chosen
# image. Supplying --image does NOT prove that image came from the compiled prompt.
#
#   ./scripts/derive-image-formats.sh <request.json> <out-dir> [--image <existing.png>]
#
# Without --image the script only compiles prompt.txt. With --image it records the source
# as externally supplied/unverified, normalizes it to artifact.png, then derives formats.

set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

resolve_promptgen() {
  if [[ -n ${PROMPTGEN:-} ]]; then
    printf '%s\n' "$PROMPTGEN"
  elif [[ -x "$ROOT/target/release/promptgen" ]]; then
    printf '%s\n' "$ROOT/target/release/promptgen"
  elif command -v promptgen >/dev/null 2>&1; then
    command -v promptgen
  else
    printf '%s\n' "$ROOT/target/release/promptgen"
  fi
}

resolve_perfectpixel() {
  if [[ -n ${PERFECTPIXEL:-} ]]; then
    printf '%s\n' "$PERFECTPIXEL"
  elif command -v perfectpixel >/dev/null 2>&1; then
    command -v perfectpixel
  elif [[ -x "$ROOT/../perfectpixel/target/release/perfectpixel" ]]; then
    printf '%s\n' "$ROOT/../perfectpixel/target/release/perfectpixel"
  else
    printf '%s\n' "perfectpixel"
  fi
}

PROMPTGEN=$(resolve_promptgen)
PERFECTPIXEL=$(resolve_perfectpixel)

die() { printf '%s\n' "$*" >&2; exit 1; }

[[ $# -ge 2 ]] || die "usage: $0 <request.json> <out-dir> [--image <existing.png>]"
REQUEST=$1
OUT=$2
shift 2

SUPPLIED_IMAGE=""
while [[ $# -gt 0 ]]; do
  case $1 in
    --image)
      [[ $# -ge 2 ]] || die "--image needs a path"
      SUPPLIED_IMAGE=$2
      shift 2
      ;;
    *) die "unknown argument: $1" ;;
  esac
done

[[ -f "$REQUEST" ]] || die "request not found: $REQUEST"
[[ -x "$PROMPTGEN" ]] || die "promptgen binary not found: $PROMPTGEN (build it or set PROMPTGEN)"
mkdir -p "$OUT"

echo "[1/3] compile prompt"
PROMPT_TMP="$OUT/.prompt.txt.partial"
rm -f "$PROMPT_TMP"
set +e
"$PROMPTGEN" image --input "$REQUEST" --mode prompt-only --format text > "$PROMPT_TMP"
compile_status=$?
set -e
if [[ $compile_status -ne 0 ]]; then
  echo "      request rejected; diagnostics:" >&2
  sed 's/^/      /' "$PROMPT_TMP" >&2
  rm -f "$PROMPT_TMP"
  exit "$compile_status"
fi
mv "$PROMPT_TMP" "$OUT/prompt.txt"
echo "      $OUT/prompt.txt ($(wc -c < "$OUT/prompt.txt" | tr -d ' ') bytes)"

echo "[2/3] obtain the image artifact"
if [[ -z "$SUPPLIED_IMAGE" ]]; then
  echo "      skipped: no --image supplied and provider generation was not requested"
  echo "      to generate: $PROMPTGEN image --input $REQUEST --mode codex-imagegen \\"
  echo "                     --image-output $OUT/artifact.png --result $OUT/receipt.json"
  exit 0
fi

[[ -f "$SUPPLIED_IMAGE" ]] || die "supplied image not found: $SUPPLIED_IMAGE"
if [[ "$PERFECTPIXEL" == */* ]]; then
  [[ -x "$PERFECTPIXEL" ]] || die "perfectpixel binary not found: $PERFECTPIXEL (set PERFECTPIXEL)"
else
  command -v "$PERFECTPIXEL" >/dev/null 2>&1 || die "perfectpixel binary not found (set PERFECTPIXEL)"
fi

# The caller supplied this image independently of promptGen. Record that fact rather than
# manufacturing prompt provenance. A real generated artifact should carry its provider receipt.
python3 - "$SUPPLIED_IMAGE" "$OUT/artifact-source.json" <<'PY'
import json
import os
import sys
source, destination = sys.argv[1:3]
payload = {
    "kind": "externally_supplied_image",
    "path": os.path.abspath(source),
    "prompt_binding": "unverified",
}
with open(destination, "w", encoding="utf-8") as handle:
    json.dump(payload, handle, ensure_ascii=False, sort_keys=True, separators=(",", ":"))
    handle.write("\n")
PY

"$PERFECTPIXEL" convert "$SUPPLIED_IMAGE" --out "$OUT/artifact.png" > /dev/null
echo "      $OUT/artifact.png (external source; prompt binding unverified)"

echo "[3/3] derive delivery formats"
"$PERFECTPIXEL" inspect "$OUT/artifact.png" > "$OUT/inspect.json"
"$PERFECTPIXEL" convert "$OUT/artifact.png" --out "$OUT/artifact.webp" > /dev/null
"$PERFECTPIXEL" convert "$OUT/artifact.png" --out "$OUT/artifact-512.webp" --width 512 > /dev/null
# JPEG has no alpha channel, so produce it only when perfectpixel reports fully opaque pixels.
alpha_ratio=$(python3 - "$OUT/inspect.json" <<'PY'
import json
import sys
with open(sys.argv[1], encoding="utf-8") as handle:
    value = json.load(handle)["alphaRatio"]
print(value)
PY
)
if [[ "$alpha_ratio" == "1.0" || "$alpha_ratio" == "1" ]]; then
  "$PERFECTPIXEL" convert "$OUT/artifact.png" --out "$OUT/artifact.jpg" --jpeg-quality 90 > /dev/null
fi

printf '\n%-24s %10s\n' "artifact" "size"
for f in "$OUT"/artifact*.png "$OUT"/artifact*.webp "$OUT"/artifact*.jpg; do
  if [[ -f "$f" ]]; then
    bytes=$(wc -c < "$f" | tr -d ' ')
    printf '%-24s %9dKB\n' "$(basename "$f")" $(( bytes / 1024 ))
  fi
done
echo
echo "pipeline complete: $OUT"
