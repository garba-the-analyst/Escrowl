# Escrowl — Architecture

Milestone-based, dispute-aware SPL escrow on Solana (Anchor/Rust).

## Diagram

```
Next.js (wallet-adapter + Anchor TS + TanStack Query)
  | tx sign/send | read getProgramAccounts
  v
Anchor program `escrowl` --CPI--> SPL Token classic
  | emit! events
  v
MVP: direct RPC reads. P2: Helius webhook -> Postgres (Supabase)
```

## Accounts / PDAs

| Account | Seeds | Authority | Purpose |
|---|---|---|---|
| Config | `["config"]` | admin | fee_bps<=500, treasury, paused, allowed_mints[4] |
| Escrow | `["escrow", buyer, seller, escrow_id_le]` | self | terms, milestones[8], totals, fee + treasury snapshots, windows, deadlines, status |
| Vault | `["vault", escrow]` | Escrow PDA | SPL token custody |

Mint: SPL classic only. Token-2022 rejected. Mint must be in allowlist.

## State machine

```
Created -fund-> Funded -submit-> Submitted -approve/timeout-> Released
                |                   |-dispute-> Disputed -resolve-> Resolved
                |                                            \-expire(timeout)-> Resolved (50/50)
                |-submit…never (seller gone)-> Pending -reclaim(deadline)-> Refunded (+ later Pending)
                cancel*-> Cancelled (refund if Funded, bare mark if Created)
*only if zero milestones ever Submitted. Completed when all terminal.
```

## Inactivity clocks (liveness, I11 — exact definitions)

- `funded_at`: set once by `fund_escrow`.
- `terminal_at[i]`: set when milestone i reaches a terminal state.
- `disputed_at[i]`: set by `raise_dispute`.
- Milestone i is "active" (reclaimable) iff it is Pending and every prior
  milestone is terminal; its clock runs from `funded_at` (i = 0) or
  `terminal_at[i-1]` (i > 0). `reclaim_stale_milestone(i)` requires
  `now >= active_since + seller_deadline_secs` and refunds milestone i plus
  all later Pending milestones in one instruction (no fee: nothing delivered).
- `expire_dispute(i)` requires Disputed and
  `now >= disputed_at + arbiter_timeout_secs`; either buyer or seller may
  call. Split is exactly 50/50 with the odd unit to the buyer and the fee on
  the seller leg only (same accounting as `resolve_dispute`).

## Instructions

initialize_config, set_paused, update_config (admin: rotate fee/treasury,
append allowlist; open escrows unaffected — fee AND treasury snapshotted),
create_escrow (terms + seller_deadline_secs + arbiter_timeout_secs),
fund_escrow, submit_milestone, approve_milestone, claim_after_timeout,
raise_dispute, resolve_dispute, reclaim_stale_milestone, expire_dispute,
cancel_escrow, close_escrow (guard: status Completed|Cancelled only).

Pause blocks create/fund only — never withdrawals/refunds/resolutions.

## Fee

`fee = floor(amount * fee_bps / 10_000)`, dust to seller. Fee AND treasury
are snapshotted at creation: later Config rotations only affect future
escrows. Fee applies to seller leg only in dispute resolution/expiry.
