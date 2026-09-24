#!/usr/bin/env bash
# Full local verification in one command. Exits non-zero on ANY failure.
# Mirrors .github/workflows/ci.yml. Requires pinned toolchains:
#   bash scripts/setup-toolchain.sh
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="$HOME/.local/bin:$HOME/.local/share/solana/bin:$HOME/.cargo/bin:$PATH"

step() { echo "=== $1 ==="; }

step "fmt"
cargo fmt --all -- --check

step "clippy (deny flags)"
cargo clippy --all-targets --locked -- -D clippy::all -D clippy::unwrap_used -D clippy::expect_used

step "rust unit + model tests"
cargo test --locked -p escrowl

step "anchor build"
anchor build

step "no driftsort in shipped binary"
if readelf -s target/sbpf-solana-solana/release/escrowl.so | grep -qi driftsort; then
  echo "driftsort present in shipped binary" >&2
  exit 1
fi
echo "driftsort absent: OK"

step "LiteSVM real-program tests"
RUSTUP_TOOLCHAIN=stable cargo test --locked --manifest-path tests-litesvm/Cargo.toml

step "anchor integration tests (localnet)"
anchor test --provider.cluster localnet --skip-build

step "tsc (sdk, scripts, tests)"
./node_modules/.bin/tsc --noEmit -p tsconfig.json

step "frontend build"
npm run build --prefix app

step "cargo audit"
cargo audit

step "cargo deny"
cargo deny check

echo
echo "ALL CHECKS PASSED"
