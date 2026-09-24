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
| Escrow | `["escrow", buyer, seller, escrow_id_le]` | self | terms, milestones[8], totals, fee snapshot, window, status |
| Vault | `["vault", escrow]` | Escrow PDA | SPL token custody |

Mint: SPL classic only. Token-2022 rejected. Mint must be in allowlist.

## State machine

```
Created -fund-> Funded -submit-> Submitted -approve/timeout-> Released
                |                   |-dispute-> Disputed -resolve-> Resolved
                cancel*-> Cancelled (refund)
*only if zero milestones ever Submitted. Completed when all terminal.
```

## Instructions

initialize_config, set_paused, update_config (admin: rotate fee/treasury,
append allowlist; open escrows unaffected), create_escrow, fund_escrow,
submit_milestone, approve_milestone, claim_after_timeout, raise_dispute,
resolve_dispute, cancel_escrow, close_escrow.

Pause blocks create/fund only — never withdrawals/refunds/resolutions.

## Fee

`fee = floor(amount * fee_bps / 10_000)`, dust to seller. Snapshot at creation.
Fee applies to seller leg only in dispute resolution.
