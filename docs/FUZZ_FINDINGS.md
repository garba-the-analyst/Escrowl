# Fuzz Findings

Trident fuzz harness asserts I1-I10 across random instruction sequences.

| ID | Finding | Severity | Fix | Test |
|---|---|---|---|---|
| F-01 | I1 as originally stated ("vault == locked at all times") is false in `Created`: vault is empty pre-deposit while milestones are Pending. Found by model fuzz (`fuzz_state_machine_seeds`). | Spec (low — no fund movement pre-funding) | Scoped I1 to Funded state; model asserts vault==0 when unfunded. On-chain behavior unchanged and correct. | `model_fuzz::fuzz_state_machine_seeds` |
| _TBD_ | Trident on-chain run (Phase 4, dev machine) | — | — | `trident-tests/` |

## Model fuzz results (2026-09-23, `cargo test -p escrowl --locked`)

- 15/15 unit tests pass (fee math, validation bounds, terminal/locked partition).
- 3/3 model fuzz tests pass: 200 seeds × 400 ops with per-op invariant
  checks, 50/50 targeted timeout-claim success, 50/50 late-dispute rejection.
- No fund-conservation violation observed across ~80k randomized transitions
  plus deliberate double-transition hammering.
