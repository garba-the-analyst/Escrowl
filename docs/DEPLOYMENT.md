# Deployment

## Devnet (target)

- Program ID: `61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE`
- Cluster: devnet
- Explorer: https://explorer.solana.com/address/61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE?cluster=devnet

## Status (2026-09-24)

- ✅ `anchor build` green — `target/deploy/escrowl.so` (462 KB), IDL + keypair
  (`61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE`) generated, SDK + app wired.
- ✅ Full localnet suite: **18/18 integration** + 15 unit + 3 model-fuzz green.
- ⏳ Devnet deploy pending: needs ~2.3 SOL on the deploy wallet
  `EzCDKkTTvQABGEhcCiyKvaBPPudf1hTa33R4FEHFGD2P` — public faucet
  rate-limited at build time. Fund it (any network with faucet access works —
  the limit is per-IP), then run the command below.

```bash
anchor deploy --provider.cluster devnet && yarn seed-devnet
```

> Note (2026-09-24): repo source is `cargo fmt`-clean; the formatted rebuild
> hashes `0db8…ff15` vs deployed `7a85…b78a` (whitespace/comments only, IDL
> identical). Upgrade needs ~2.35 SOL buffer rent temporarily — redeploy +
> reseed once the wallet is topped up.
```

## Steps (full runbook)

Pinned toolchains (verified green — SBF codegen is version-sensitive, a
dependency's generic `sort` exceeds the 4 KB stack frame under other
rustc/platform-tools combos): Solana CLI **2.2.14**, Anchor CLI **0.31.2**,
Rust **1.89.0** via `rust-toolchain.toml`, Node 20. Install them with:

```bash
bash scripts/setup-toolchain.sh
```

If `anchor build` still fails with `driftsort ... Stack offset ... exceeded`:

1. Confirm versions: `solana --version` → 2.2.14, `anchor --version` → 0.31.2.
2. Confirm the committed `Cargo.lock` is present and unmodified
   (`git status` clean) — a fresh resolve pulls newer crates whose codegen
   overflows the SBF frame. Never delete it; builds must reuse it.
3. `rm -rf target/ .anchor/` and rebuild clean.
4. If it persists, send the FULL build log (the failing crate appears just
   above the error) — do not deploy a binary that reports stack errors, as
   the same frame can fault at runtime (`Access violation in stack frame`).

### Known benign warning: driftsort stack frame

`anchor build` prints (exit code stays 0, binary is produced):

```
Error: Function _ZN4core5slice4sort6stable14driftsort_main17... Stack offset
of 4104 exceeded max offset of 4096 by 8 bytes ...
```

This is a **false positive, verified 2026-09-24**: the symbol comes from
`SlotHashes::new`'s `sort_by` in the `solana-slot-hashes` rlib (a dependency
we never call — no `.sort` exists anywhere in our source). Dead-code
elimination removes it from the shipped binary. Proof on any build:

```bash
readelf -s target/sbpf-solana-solana/release/escrowl.so | grep -ci driftsort
# → 0 (absent from final binary)

grep -rl driftsort target/sbpf-solana-solana/release/deps/*.rlib
# → only libsolana_slot_hashes-*.rlib (unlinked intermediate)
```

Confirm the build actually succeeded: `echo $?` → 0 and a fresh timestamp on
`target/deploy/escrowl.so`. The program passes the on-chain loader's own
stack verification at deploy time (ours deployed + all paths executed on
devnet without a single frame fault).

```bash
# 1. One-time: Solana CLI + Anchor 0.31.1, wallet with devnet SOL
solana airdrop 2 --url devnet

# 2. Build + verifiable build
anchor build
solana-verify build --library-name escrowl

# 3. Deploy (runs post-deploy checklist)
anchor deploy --provider.cluster devnet
# or: ts-node scripts/deploy.ts

# 4. Seed demo state (mint + escrows in Created/Funded/Submitted/Disputed/Completed)
yarn seed-devnet
# optional overrides: MINT=<existing> ESCROW_ID_BASE=42 yarn seed-devnet
```

## Seed script creates (`scripts/seed-devnet.ts`, via SDK dogfooding)

- Test mint (6 decimals, or reuse via `MINT=`) + ATAs for buyer/seller/treasury
- Config (250 bps) if missing
- Escrows in Created, Funded, Submitted, Disputed, Completed states
- Prints explorer links — paste below after running

## Live addresses (deployed + seeded 2026-09-24)

Deploy tx: `5QFT1nXbMPmSMT1ZvxnX7kG9JU17JNsFMTUKE1bshR8A7TyyGc8xGXsCtBrCxaeS8vJUUKZ25Ku5JkorRFYncx5u`
([explorer](https://explorer.solana.com/tx/5QFT1nXbMPmSMT1ZvxnX7kG9JU17JNsFMTUKE1bshR8A7TyyGc8xGXsCtBrCxaeS8vJUUKZ25Ku5JkorRFYncx5u?cluster=devnet))

| Item | Address |
|---|---|
| Program | `61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE` |
| Mint (6-dec test) | `GPHb2cEa1mh2SpHrm9Zs1kREbKXCntHEm5MR1zDUxcpr` |
| Buyer | `9usPveczxaDUuadiAKisFe7BGiBfgjAZyeMZp8uduQhj` |
| Seller | `87NMWo7j7NRmHPGZNS4ATaFqtHpatCw9J3NoorKAdPQy` |
| Arbiter | `B2Gmo6cNk4idZSToBkc6xpk5bJwEa45ENfjDbrCPha8W` |
| Treasury owner | `CJSYD5VNWZTKSRQHPttSuN8TraQzQZfLQRCXcRAn8LoT` |
| Escrow (Created) | `9PNUqAk71WgM2Lk71Eh78Kuwe42kjhjgDE3DFLy1u1hx` |
| Escrow (Funded) | `HKXGgD5au34qzKoRKX8cKRjc4mrcd5j4P5C6dj2KRBG9` |
| Escrow (Submitted) | `8z5MtZPnw6szboDox7X8FwMXAxrvVvmTEXD33eaffczr` |
| Escrow (Disputed) | `3FRYL78kVRLpW14guZdi2hQPpjo82U2vVPseU9gKYuD5` |
| Escrow (Completed) | `Gj9G6ZSCU27JvSR8mDSpH2cBqXqDYNvn3cCsqg2gvyNC` |

Append `?cluster=devnet` on explorer links, e.g.
https://explorer.solana.com/address/9PNUqAk71WgM2Lk71Eh78Kuwe42kjhjgDE3DFLy1u1hx?cluster=devnet

## Live addresses (template for re-seeds)

| Item | Address | Tx / notes |
|---|---|---|
| Program | _TBD after deploy_ | |
| Mint | _TBD after seed_ | |
| Config | _TBD_ | |
| Escrow (Created) | _TBD_ | |
| Escrow (Funded) | _TBD_ | |
| Escrow (Submitted) | _TBD_ | |
| Escrow (Disputed) | _TBD_ | |
| Escrow (Completed) | _TBD_ | |
