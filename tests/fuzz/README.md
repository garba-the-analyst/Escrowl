# Fuzzing

Two layers. Both assert docs/INVARIANTS.md.

## Layer 1 — model fuzz (runs anywhere, no validator)

`programs/escrowl/tests/model_fuzz.rs` — dependency-free, deterministic
(xorshift64) state-machine fuzz mirroring the on-chain transition rules:

- `fuzz_state_machine_seeds`: 200 seeds × 400 random ops (submit / approve /
  timeout-claim / dispute / resolve with exact AND inexact splits / clock
  warp / deliberate double-transition hammering). After every op: I1
  (vault == locked, scoped to Funded), fund conservation
  (vault + buyer + seller + treasury == total), released + refunded + locked
  == total, per-payout fee + proceeds == leg (I5).
- `fuzz_timeout_claim_path_reached`: warps past the review window, proves the
  timeout-claim success path conserves funds (the path localnet TS tests
  cannot reach — localnet clock follows wall time, min window is 1h).
- `fuzz_late_dispute_rejected`: dispute at/exactly past deadline always fails.

```bash
cargo test -p escrowl --locked --test model_fuzz
```

## Layer 2 — on-chain fuzz with Trident (needs Solana + Anchor toolchains)

Trident executes the real program against random instruction sequences,
including CPI-level faults (fake vault, wrong mint/owner, PDA substitution)
that the model cannot express.

```bash
# one-time setup (dev machine with Solana CLI + Anchor 0.31.1)
cargo install trident-cli --locked
trident init          # scaffolds trident-tests/ (do NOT hand-write the API)
# Implement per-instruction flows + invariants I1-I10 in trident-tests/,
# mirroring tests/integration/ account shapes, then:
trident fuzz --timeout 600
```

Fuzz budget guidance: start 10 min / 1M transactions for iteration,
final pre-submission run 60 min. Every crash → minimize, fix root cause in
the program (never weaken the invariant), log below in docs/FUZZ_FINDINGS.md
with the Trident seed for replay.
