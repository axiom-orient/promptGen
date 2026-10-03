#!/usr/bin/env bash
set -euo pipefail

ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$ROOT"
export CARGO_INCREMENTAL=0
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }-Dwarnings"
export RUSTDOCFLAGS="${RUSTDOCFLAGS:+$RUSTDOCFLAGS }-Dwarnings"

cargo check --workspace --all-targets --locked --offline
cargo test --workspace --all-targets --locked --offline
cargo test --doc --workspace --locked --offline
./scripts/verify-quality.sh
cargo doc --workspace --no-deps --locked --offline
cargo build --release --workspace --locked --offline
./scripts/generate-schemas.sh --check "${CARGO_TARGET_DIR:-$ROOT/target}/release/promptgen"
python3 ./scripts/audit-architecture.py
python3 ./scripts/audit-prompts.py
if [ ! -e evals/dogfood/historical/1.4.0/image/image-evidence.json ] \
  && [ ! -e evals/dogfood/historical/1.4.0/image/IMAGE_RESULT_ANALYSIS.md ] \
  && [ ! -e evals/dogfood/current/IMAGE_EVIDENCE_POLICY.md ]; then
  export PROMPTGEN_VERIFICATION_NOT_RUN='historical image evidence audit: archived evidence inputs are absent'
fi
python3 ./scripts/audit-image-evidence.py
python3 ./scripts/audit-doc-links.py
python3 ./scripts/audit-catalog-assets.py
python3 ./scripts/audit-representative-image-qa.py
python3 ./scripts/renew-image-catalog.py --check
python3 ./scripts/generate-catalog-image-prompts.py --check
python3 ./scripts/audit-rust-structure.py
python3 ./scripts/test-source-snapshot.py
python3 ./scripts/test-codex-version.py
python3 -c 'import ast, pathlib; [ast.parse(path.read_text(encoding="utf-8"), filename=str(path)) for path in pathlib.Path("scripts").glob("*.py")]'
for script in ./scripts/*.sh; do bash -n "$script"; done
if command -v node >/dev/null 2>&1; then node --check crates/promptgen-web/src/assets/app.js; fi
./scripts/smoke.sh "${CARGO_TARGET_DIR:-$ROOT/target}/release/promptgen"
python3 ./scripts/smoke-web.py "${CARGO_TARGET_DIR:-$ROOT/target}/release/promptgen"
python3 ./scripts/source_snapshot.py --record-verification
echo "[OK] image-only promptGen verification passed"
