# Escrowl

[![CI](https://github.com/garba-the-analyst/Escrowl/actions/workflows/ci.yml/badge.svg)](https://github.com/garba-the-analyst/Escrowl/actions/workflows/ci.yml)
[![Anchor](https://img.shields.io/badge/anchor-0.31.1-blueviolet)](Anchor.toml)
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

Threat model + 10 invariants + negative/role/fuzz tests. See `docs/THREAT_MODEL.md`, `docs/INVARIANTS.md`, `docs/SECURITY.md`.

### Security self-audit (run before every release)

```bash
cargo clippy --all-targets --locked   # workspace lints: clippy::all deny, no unwrap/expect, unsafe forbid
cargo fmt --check
cargo audit                                  # no known vulnerable deps (Cargo.lock pinned)
cargo deny check licenses bans sources
cargo test -p escrowl --locked               # 15 unit + 3 model-fuzz suites
trident fuzz --timeout 3600                  # on-chain fuzz, log in docs/FUZZ_FINDINGS.md
solana-verify build --library-name escrowl   # reproducible build matches deploy
```

Latest results: clippy clean (safety lints), `cargo test` 18/18 green,
model fuzz ~80k randomized transitions with zero conservation violations,
1 spec finding fixed (F-01, I1 scoping). Details in `docs/FUZZ_FINDINGS.md`.
On-chain Trident run happens on the dev machine (see `tests/fuzz/README.md`).

## Demo (3 minutes)

Script in `docs/DEMO.md` — seeded devnet walkthrough: create → fund →
submit/approve with fee split → dispute + exact-split resolve → close.

## Program ID (devnet)

`61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE` — [explorer](https://explorer.solana.com/address/61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE?cluster=devnet) (see `docs/DEPLOYMENT.md`)

## Roadmap

Token-2022 support, multi-arbiter, reputation, Helius indexer, mainnet.

## Bounty

Submitted to Colosseum Crypto World's Fair; applied to Adevar Labs $20k Pre-Audit track on Superteam Earn.
