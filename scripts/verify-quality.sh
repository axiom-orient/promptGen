#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"

for tool in cargo rustfmt cargo-clippy; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "[UNAVAILABLE] $tool is required for quality verification" >&2
    exit 127
  fi
done

cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
