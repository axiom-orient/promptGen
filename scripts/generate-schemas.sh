#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
CHECK=0
if [[ "${1:-}" == "--check" ]]; then
  CHECK=1
  shift
fi
BIN=${1:-${PROMPTGEN_BIN:-$ROOT/target/debug/promptgen}}
[[ $# -le 1 ]] || { echo "usage: $0 [--check] [promptgen-binary]" >&2; exit 2; }
[[ -x "$BIN" ]] || { echo "[FAIL] build promptgen before generating schemas" >&2; exit 1; }
TMP=$(mktemp -d "${TMPDIR:-/tmp}/promptgen-schemas.XXXXXX")
trap 'rm -rf "$TMP"' EXIT
targets=(image interview compilation)
for target in "${targets[@]}"; do
  generated="$TMP/$target.schema.json"
  checked_in="$ROOT/schemas/$target.schema.json"
  "$BIN" schema "$target" >"$generated"
  python3 - "$generated" <<'PY'
import json, pathlib, sys
value = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert value.get("$schema") == "https://json-schema.org/draft/2020-12/schema"
PY
  if (( CHECK )); then
    if ! cmp -s "$generated" "$checked_in"; then
      echo "[FAIL] checked-in schema drift: schemas/$target.schema.json" >&2
      diff -u "$checked_in" "$generated" >&2 || true
      exit 1
    fi
  else
    install -m 0644 "$generated" "$checked_in"
  fi
done
if (( CHECK )); then
  echo "[OK] checked-in ${#targets[@]} image schemas match runtime output"
else
  echo "[OK] generated ${#targets[@]} image schemas"
fi
