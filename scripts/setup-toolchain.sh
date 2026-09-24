#!/usr/bin/env bash
# Install the EXACT pinned toolchains Escrowl is verified against.
# SBF codegen is version-sensitive: other Solana/Anchor combos can fail the
# build with stack-frame errors (e.g. driftsort exceeding the 4 KB frame)
# even though the source is identical. Run this before anything else.
#
#   bash scripts/setup-toolchain.sh
#
# Sources are GitHub release binaries (no compilation, ~5 min on decent net).
set -euo pipefail

SOLANA_VERSION="2.2.14"
ANCHOR_VERSION="0.31.2"
INSTALL_DIR="${HOME}/.local"

export PATH="${INSTALL_DIR}/bin:${INSTALL_DIR}/share/solana/bin:${HOME}/.cargo/bin:${PATH}"

need() { command -v "$1" >/dev/null 2>&1 || { echo "missing required tool: $1" >&2; exit 1; }; }
need curl
need tar

# --- Solana CLI (Agave release tarball) ---
if solana --version 2>/dev/null | grep -q "${SOLANA_VERSION}"; then
  echo "solana ${SOLANA_VERSION} already installed"
else
  echo "installing solana ${SOLANA_VERSION} ..."
  mkdir -p /tmp/solana-dl "${INSTALL_DIR}/share/solana"
  curl -sSfL "https://github.com/anza-xyz/agave/releases/download/v${SOLANA_VERSION}/solana-release-x86_64-unknown-linux-gnu.tar.bz2" \
    -o /tmp/solana-dl/solana.tar.bz2
  tar -xjf /tmp/solana-dl/solana.tar.bz2 -C "${INSTALL_DIR}/share/solana" --strip-components=1
  rm -f /tmp/solana-dl/solana.tar.bz2
fi

# --- Anchor CLI (prebuilt binary, no avm needed) ---
mkdir -p "${INSTALL_DIR}/bin"
if anchor --version 2>/dev/null | grep -q "${ANCHOR_VERSION}"; then
  echo "anchor ${ANCHOR_VERSION} already installed"
else
  echo "installing anchor ${ANCHOR_VERSION} ..."
  curl -sSfL "https://github.com/coral-xyz/anchor/releases/download/v${ANCHOR_VERSION}/anchor-${ANCHOR_VERSION}-x86_64-unknown-linux-gnu" \
    -o "${INSTALL_DIR}/bin/anchor"
  chmod +x "${INSTALL_DIR}/bin/anchor"
fi

echo
echo "PATH add (persist in ~/.bashrc if needed):"
echo "  export PATH=\"\$HOME/.local/bin:\$HOME/.local/share/solana/bin:\$HOME/.cargo/bin:\$PATH\""
echo
solana --version
anchor --version
echo
echo "Next: anchor build   (uses rust-toolchain.toml + committed Cargo.lock — do not delete it)"
