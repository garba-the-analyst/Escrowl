# Fuzz Findings

## Model checks (`programs/escrowl/tests/model_fuzz.rs`)

Validates fee math, validation bounds, and state predicates (real crate
code) plus a mirrored transition model. This is NOT program fuzzing.

| ID | Finding | Severity | Status |
|---|---|---|---|
| F-01 | I1 as originally stated ("vault == locked at all times") is false in `Created`: vault is empty pre-deposit while milestones are Pending | Spec (low) | Fixed: I1 scoped to Funded; model asserts vault==0 when unfunded |

## LiteSVM real-program fuzz (`tests-litesvm/tests/fuzz.rs`, 2026-09-24)

200 seeds x 100 ops (4 shards x 50, parallel) = 20,000 randomized valid AND
invalid instruction sequences against the compiled `target/deploy/escrowl.so`,
with random clock warps. After every accepted op: exact conservation
(vault+seller+buyer+treasury == funded) and released+refunded+locked+
cancelled == total; after every rejected op: state-hash bit-equality
(atomicity). Result: **all green, zero conservation violations**.

Findings during development (all fixed, all with regression tests):

| ID | Finding | Severity | Fix | Test |
|---|---|---|---|---|
| F-02 | I1 ledger equation missed the new `Cancelled` status (no-fund cancel) | Test bug | 4-term ledger equation | fuzz seed 1 |
| F-03 | Rejected-path atomicity relies on runtime revert; verified, not assumed | Info | state-hash assert on every rejection | all fuzz ops |
| F-04 | Fee test approved index 0 twice (authored bug, program correctly rejected double-release with InvalidMilestoneState) | Test bug | approve index 1 | `fee_edges_max_fee_and_dust` |

## Trident

Future work: on-chain Trident fuzz per `tests/fuzz/README.md` runbook.
Not run in this environment (needs the full Solana toolchain harness).
