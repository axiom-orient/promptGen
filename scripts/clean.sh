#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
DRY_RUN=0
REMOVE_GENERATED_OUTPUT=0

usage() {
  echo "usage: $0 [--dry-run] [--generated-output]"
}

while [[ $# -gt 0 ]]; do
  case $1 in
    --dry-run) DRY_RUN=1 ;;
    --generated-output) REMOVE_GENERATED_OUTPUT=1 ;;
    -h|--help) usage; exit 0 ;;
    *) usage >&2; exit 2 ;;
  esac
  shift
done

if [[ ! -f "$ROOT/Cargo.toml" ]]; then
  echo "not a promptGen repository: $ROOT" >&2
  exit 2
fi

remove_path() {
  local path=$1
  [[ -e $path || -L $path ]] || return 0
  case $path in
    "$ROOT"/*) ;;
    *) echo "refusing to remove outside repository: $path" >&2; exit 2 ;;
  esac
  if (( DRY_RUN )); then
    echo "[DRY-RUN] remove ${path#"$ROOT/"}"
  else
    rm -rf "$path"
    echo "[REMOVED] ${path#"$ROOT/"}"
  fi
}

# Rebuildable product/build outputs and retired root identity files.
for relative in target dist SOURCE_MANIFEST.sha256 SOURCE_SNAPSHOT.json; do
  remove_path "$ROOT/$relative"
done

# Generated images are product/user evidence, not rebuildable cache. Remove them only
# when the caller explicitly asks for destructive generated-output cleanup.
if (( REMOVE_GENERATED_OUTPUT )); then
  remove_path "$ROOT/var/output"
fi

# Local Python/editor/test noise is always disposable.
while IFS= read -r path; do
  remove_path "$path"
done < <(
  find "$ROOT" \
    \( -path "$ROOT/.git" -o -path "$ROOT/target" -o -path "$ROOT/var" \) -prune -o \
    \( -type d \( -name __pycache__ -o -name .pytest_cache -o -name .mypy_cache -o -name .ruff_cache \) \
       -o -type f \( -name '*.pyc' -o -name '*.pyo' -o -name '.DS_Store' -o -name '*.tmp' -o -name '*.bak' \) \) -print
)

# Remove reinstallable dependencies only when a matching restore manifest is adjacent.
while IFS= read -r path; do
  parent=${path%/*}
  name=${path##*/}
  case $name in
    node_modules)
      [[ -f "$parent/package.json" || -f "$parent/package-lock.json" || -f "$parent/pnpm-lock.yaml" || -f "$parent/yarn.lock" ]] || continue
      ;;
    vendor)
      [[ -f "$parent/composer.json" || -f "$parent/composer.lock" ]] || continue
      ;;
    .venv|venv)
      [[ -f "$parent/pyproject.toml" || -f "$parent/requirements.txt" || -f "$parent/setup.py" || -f "$parent/tox.ini" ]] || continue
      ;;
    *)
      continue
      ;;
  esac
  remove_path "$path"
done < <(
  find "$ROOT" \
    \( -path "$ROOT/.git" -o -path "$ROOT/target" -o -path "$ROOT/var" \) -prune -o \
    -type d \( -name vendor -o -name node_modules -o -name .venv -o -name venv \) -print
)

if (( DRY_RUN )); then
  echo "[OK] dry-run complete"
else
  if (( REMOVE_GENERATED_OUTPUT )); then
    echo "[OK] repository artifacts cleaned, including explicitly requested generated output"
  else
    echo "[OK] repository artifacts cleaned; generated output, lockfiles, source, docs, catalog, and eval evidence preserved"
  fi
fi
