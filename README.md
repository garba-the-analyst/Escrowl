# Escrowl

[![CI](https://github.com/garba-the-analyst/Escrowl/actions/workflows/ci.yml/badge.svg)](https://github.com/garba-the-analyst/Escrowl/actions/workflows/ci.yml)
[![Anchor](https://img.shields.io/badge/anchor-0.31.2-blueviolet)](Anchor.toml)
[![Solana](https://img.shields.io/badge/solana-devnet-green)](docs/DEPLOYMENT.md)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Milestone-based, dispute-aware SPL escrow on Solana — built for Colosseum Crypto World's Fair + Adevar Labs Pre-Audit side track.

## Why

Freelance / P2P payments need trust without intermediaries holding keys. Escrowl is the reusable custody primitive: buyer funds a PDA vault, seller delivers per-milestone, arbiter resolves disputes with exact splits.

## Architecture

See `docs/ARCHITECTURE.md`. PDAs: Config `["config"]`, Escrow `["escrow",buyer,seller,id]`, Vault `["vault",escrow]`. SPL classic only.

## Quickstart

```bash
# 0. Exact pinned toolchains — REQUIRED, SBF builds are version-sensitive:
bash scripts/setup-toolchain.sh   # Solana 2.2.14 + Anchor 0.31.2 + Rust 1.89
export PATH="$HOME/.local/bin:$HOME/.local/share/solana/bin:$HOME/.cargo/bin:$PATH"
# then Node 20 + Yarn for the TS side.
# 1. Deps
npm install --no-audit --no-fund && cd app && npm install && cd ..
# 2. Build
anchor build
# 3. Test
anchor test
cargo clippy --all-targets --locked
# 4. Devnet
anchor deploy --provider.cluster devnet
yarn seed-devnet
# 5. Frontend
cd app && npm install && npm run dev  # set NEXT_PUBLIC_PROGRAM_ID in .env.local
```

## Security posture

Threat model + 12 invariants (I1–I12) + role/adversarial/fuzz tests — all
proven against the real program unless marked otherwise. See
`docs/THREAT_MODEL.md`, `docs/INVARIANTS.md`, `docs/SECURITY.md`,
`docs/SELF_AUDIT.md`, `docs/AUDIT_PREP.md`.

> Escrowl has NOT been externally audited. The arbiter is a trusted role and
> the admin key is trusted pre-mainnet (see Trust model in AUDIT_PREP.md).

### Security self-audit (run before every release)

```bash
bash scripts/verify-repo.sh   # everything below in one command (non-zero on failure)
```

What it runs: `cargo fmt --check`; `cargo clippy` with deny flags;
`cargo test` (16 unit + 3 model); `anchor build` + no-driftsort check;
LiteSVM real-program suites (23 tests incl. 200×100 randomized
conservation fuzz); `anchor test` (19 integration); `tsc --noEmit`;
`next build`; `cargo audit`; `cargo deny check`.

## What is and is not verified (2026-09-24)

- Verified here, with output: 16 Rust unit · 3 model checks · 19 Anchor
  integration (localnet) · 23 LiteSVM real-program tests (9 adversarial,
  4 lifecycle, 4 liveness, 1 smoke, 4 fuzz shards of 50 seeds × 100 ops);
  clippy deny-flags clean; fmt clean; `anchor build` green; `next build`
  green; devnet deployment + seeded escrows in every state.
- NOT done here: external audit (none — this repo is applying for one);
  Trident on-chain fuzz (runbook in `tests/fuzz/README.md`, pending);
  `solana-verify` reproducible build (pending).
- Model checks (`programs/escrowl/tests/model_fuzz.rs`) validate fee math
  and a mirrored state model — they are NOT program fuzzing. Real
  randomized coverage is the LiteSVM fuzz above.

## Demo (3 minutes)

Script in `docs/DEMO.md` — seeded devnet walkthrough: create → fund →
submit/approve with fee split → dispute + exact-split resolve → close.

## Program ID (devnet)

`61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE` — [explorer](https://explorer.solana.com/address/61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE?cluster=devnet) (see `docs/DEPLOYMENT.md`)

## Roadmap

Token-2022 support, multi-arbiter, reputation, Helius indexer, mainnet.

## Bounty

Built for Colosseum Crypto World's Fair (submit before 12 Oct 2026) and the
Adevar Labs $20k Pre-Audit side track on Superteam Earn. Submission status
is tracked off-repo — confirm on the respective platforms.
