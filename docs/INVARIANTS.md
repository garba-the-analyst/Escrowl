# Escrowl — Invariants (must each have a passing test)

- I1 vault balance == sum amounts of milestones in {Pending, Submitted, Disputed},
  whenever the escrow is Funded. (In Created the vault is legitimately empty
  pre-deposit; after full terminal drain both are zero. Found by model fuzz —
  see docs/FUZZ_FINDINGS.md F-01.)
- I2 role checks: only buyer approves/disputes/cancels; only seller submits/claims; only arbiter resolves.
- I3 each milestone reaches terminal at most once (no double payout).
- I4 dispute splits sum exactly to milestone amount.
- I5 fee = floor(amount * fee_bps / 10_000); remainder to seller; fee snapshot at create.
- I6 buyer, seller, arbiter pairwise distinct.
- I7 sum(milestones) == total_amount; each > 0; count 1..=8.
- I8 mint in allowlist + owned by classic Token program.
- I9 pause never blocks exits (approve/claim/resolve/cancel/close work while paused).
- I10 closing zeroes data, vault empty, rent to buyer.

## Test mapping (Phase 3)

| Invariant | Rust unit (`cargo test -p escrowl`) | TS integration (`anchor test`) |
|---|---|---|
| I1 | — (on-chain, asserted in `fund_escrow`) | `01` fund vault==total; approve leaves vault==m1 |
| I2 | — | `02` non-arbiter resolve, seller re-dispute; `03` wrong-role submit/approve, non-admin pause |
| I3 | `terminal_and_locked_partition_all_states` | `01` double-fund, double-approve; `03` double submit |
| I4 | — | `02` off-by-one/overshoot splits rejected, 60/40 + full-refund resolve |
| I5 | `fee_standard_250bps`, `fee_floors_and_dust_goes_to_seller`, `fee_u64_max_does_not_overflow` | `01` seller/treasury balance deltas == split |
| I6 | `distinct_roles_ok`, `duplicate_roles_rejected` | `03` buyer==seller, buyer==arbiter, seller==arbiter |
| I7 | `milestone_counts` | `01` create asserts totals; `03` empty/9/zero-amount sets |
| I8 | `allowlist_bounds` | `03` foreign mint rejected (Token-2022 rejection is structural: vault/mint must be classic `Account<Mint>`) |
| I9 | — | `03` pause blocks create+fund, approve succeeds while paused |
| I10 | — | `01` close after completed, fetch throws; `03` close on non-terminal fails |

Timeout-success claim (seller path past deadline) needs clock warp: covered by
LiteSVM in Phase 4. TS asserts the security-critical directions
(early claim → `WindowNotElapsed`).
