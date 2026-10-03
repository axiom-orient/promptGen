#!/usr/bin/env bash
set -euo pipefail

ROOT=${1:-$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)}
if [[ $# -gt 1 ]]; then
  echo "usage: $0 [repository-root]" >&2
  exit 2
fi
ROOT=$(CDPATH= cd -- "$ROOT" && pwd)

if [[ ! -f "$ROOT/Cargo.toml" ]]; then
  echo "not a promptGen repository: $ROOT" >&2
  exit 2
fi

size_bytes() {
  if [[ -d $1 ]]; then
    du -sk "$1" | awk '{print $1 * 1024}'
  else
    wc -c < "$1" | tr -d ' '
  fi
}

file_count() {
  if [[ -d $1 ]]; then
    find "$1" -type f | wc -l | tr -d ' '
  else
    printf '1\n'
  fi
}

tracking() {
  local relative=$1
  if ! git -C "$ROOT" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    printf 'UNKNOWN(no .git)'
  elif git -C "$ROOT" ls-files --error-unmatch -- "$relative" >/dev/null 2>&1; then
    printf 'tracked'
  else
    printf 'untracked'
  fi
}

print_candidate() {
  local path=$1 relative=${1#"$ROOT/"}
  [[ -e $path || -L $path ]] || return 0
  printf '%s\t%s\t%s\t%s\n' \
    "$relative" "$(size_bytes "$path")" "$(file_count "$path")" "$(tracking "$relative")"
}

printf 'path\tbytes\tfiles\ttracking\n'
for relative in target dist var/output SOURCE_MANIFEST.sha256 SOURCE_SNAPSHOT.json; do
  print_candidate "$ROOT/$relative"
done

while IFS= read -r path; do
  print_candidate "$path"
done < <(
  find "$ROOT" \
    \( -path "$ROOT/.git" -o -path "$ROOT/target" -o -path "$ROOT/var" \) -prune -o \
    -type d \( -name vendor -o -name node_modules -o -name .venv -o -name venv \) -print
)

while IFS= read -r path; do
  print_candidate "$path"
done < <(
  find "$ROOT" \
    \( -path "$ROOT/.git" -o -path "$ROOT/target" -o -path "$ROOT/var" \) -prune -o \
    -type d \( -name __pycache__ -o -name .pytest_cache -o -name .mypy_cache -o -name .ruff_cache \) -print
)

while IFS= read -r path; do
  print_candidate "$path"
done < <(
  find "$ROOT" \
    \( -path "$ROOT/.git" -o -path "$ROOT/target" -o -path "$ROOT/var" \) -prune -o \
    -type f \( -name '*.pyc' -o -name '*.pyo' -o -name '.DS_Store' -o -name '*.tmp' -o -name '*.bak' \) -print
)
