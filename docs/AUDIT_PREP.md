# Audit Preparation Briefing — Escrowl

For: Adevar Labs pre-audit review (Crypto World's Fair side track).
Program: `61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE` (devnet).
Repo: https://github.com/garba-the-analyst/Escrowl — build from `main` with
`bash scripts/setup-toolchain.sh && anchor build` (pinned: Solana 2.2.14,
Anchor 0.31.2, Rust 1.89.0, committed `Cargo.lock`).

## Scope

In scope — `programs/escrowl/src` (~2,200 lines Rust, `#![forbid(unsafe_code)]`):

| File(s) | Contents |
|---|---|
| `lib.rs` | 14-instruction dispatcher |
| `state/{config,escrow,milestone}.rs` | Config PDA, Escrow PDA (incl. fee/treasury snapshots, liveness windows), Milestone array |
| `instructions/*.rs` (14 files) | All handlers; each validates accounts, moves funds, flips state, emits events |
| `utils/{math,validation}.rs` | Checked fee math, input bounds |
| `errors.rs` (25 variants, 6000–6024), `events.rs` (12 events), `constants.rs` | Codes, events, seeds/bounds |

Out of scope: `sdk/` (client wrapper), `app/` (Next.js frontend), `scripts/`,
test code. External dependencies: `anchor-lang`/`anchor-spl 0.31.1`, SPL Token
program (classic only — Token-2022 is rejected), System program.

## Architecture in 60 seconds

Buyer creates an escrow (1–8 milestones, fixed amounts) and funds a PDA vault
(`["vault", escrow]`, authority = Escrow PDA). Seller submits milestones in
order; buyer approves (fee split seller/treasury) or disputes to a designated
arbiter (exact-sum split). Seller can claim after the review window; buyer can
cancel only before work starts; anyone closes when terminal + vault empty.
Liveness exits: buyer reclaims stale Pending milestones after a seller
deadline; either party expires abandoned disputes 50/50 after an arbiter
timeout. Pause blocks create/fund only — exits are never gated.

## Trust model (explicit assumptions)

1. **Arbiter is trusted** within a dispute (single arbiter chosen at creation).
   Mitigations: exact-sum enforcement (no value created), fee only on the
   seller leg, 50/50 expiry bounds abandonment loss, buyer/seller choose the
   arbiter jointly at creation.
2. **Admin key is trusted**: can pause create/fund, rotate fee/treasury for
   FUTURE escrows, extend the mint allowlist (max 4). Cannot touch open
   escrows (fee + treasury snapshotted), cannot pause exits, cannot move funds.
3. **Upgrade authority** (devnet deploy key) is trusted pre-mainnet.
   Recommendation for mainnet: multisig + timelock, or renounce.
4. **SPL Token classic + USDC issuer** trusted (no Token-2022: hooks/fees
   rejected by owner check).
5. **Clock**: `Clock::get()` trusted for all windows/deadlines (standard).

## Invariants (I1–I12, see docs/INVARIANTS.md for enforcement sites + tests)

I1 vault == locked while Funded · I2 role checks · I3 terminal-once ·
I4 exact splits · I5 floor fee + snapshot · I6 distinct roles ·
I7 milestone shape · I8 allowlist + classic · I9 pause never blocks exits ·
I10 close gating · I11 liveness (no permanent lock) · I12 conservation.

## Known issues (self-reported, all with status)

- F-01 (spec): I1 originally unstated pre-funding scope — fixed, scoped to
  Funded. See docs/FUZZ_FINDINGS.md.
- `VaultNotEmpty` (6019) is defense-in-depth: unreachable through valid
  transitions (status gating implies empty vault), kept as a second barrier.
- Build prints a benign `driftsort` stack warning (dead code in a dependency
  rlib, absent from the shipped binary — proof in docs/DEPLOYMENT.md).

## How to verify (all green 2026-09-24)

```bash
bash scripts/verify-repo.sh   # fmt, clippy deny-flags, unit+model, anchor build,
                              # no-driftsort check, LiteSVM (23 tests), anchor test (19),
                              # tsc, next build
```

Test inventory: 16 Rust unit · 3 model · 19 Anchor integration (localnet) ·
23 LiteSVM real-program (10 adversarial, 4 lifecycle, 4 liveness, 1 smoke,
4 fuzz shards of 50 seeds x 100 ops with conservation checks).
Negative tests assert SPECIFIC error codes, never just failure.

## Questions we want auditors to focus on

1. Liveness exits: is there ANY funded state from which funds cannot reach a
   terminal empty-vault state within bounded time, assuming at most one of
   {seller, arbiter} is live? (Our I11 argument is in docs/SELF_AUDIT.md.)
2. The approve-vs-dispute and claim-vs-dispute races at window boundaries —
   are the semantics (dispute iff now < deadline; claim iff now >= deadline)
   the fairest choice, and is there a slot where both or neither can act?
3. Fee/treasury snapshot completeness: is there any path where a Config
   change affects an open escrow's economics?
4. Account aliasing: any two accounts in one instruction that must differ but
   aren't constrained to (see SELF_AUDIT.md aliasing table + tests)?
5. i64 timestamp arithmetic at extremes (Clock near i64::MAX with 90-day
   windows) — checked_add covers it, but confirm the reasoning.
