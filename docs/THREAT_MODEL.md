# Escrowl — Threat Model (STRIDE)

## Actors / Assets

Actors: buyer, seller, arbiter (pairwise distinct), admin, attacker.
Assets: vault SPL funds, escrow terms, fee revenue, rent.

## Threats

| # | Attack | Mitigation | Test proving it (verified by grep) |
|---|---|---|---|
| T1 | Buyer approves twice / replay -> double payout | Terminal-once transition, status check | `double_transitions_and_cross_roles_fail`, fuzz double-transition hammering |
| T2 | Seller submits out of order / fake index | In-order check, index bounds | `rejects out-of-order submit and double submit` (TS) |
| T3 | Non-buyer approves, non-seller submits, non-arbiter resolves | has_one + signer role checks | `buyer disputes in-window; non-buyer cannot`, `non-arbiter cannot resolve` (TS), `double_transitions_and_cross_roles_fail` |
| T4 | Fake vault / wrong mint / wrong owner PDA substitution | Re-derive PDA, check mint allowlist + Token program owner | `substituted_vault_rejected_everywhere`, `account_aliasing_rejected`, `wrong_owner_token_accounts_rejected`, `rejects substituted vault / token accounts` (TS) |
| T5 | Token-2022 hook / freeze / transfer fee abuse | Reject Token-2022, classic only | `token2022_mint_cannot_create_escrow`, `rejects bad window and foreign mint` (TS) |
| T6 | Overflow fee / dust theft | checked_* math, floor fee, dust to seller | `fee_edges_max_fee_and_dust`, unit `fee_floors_and_dust_goes_to_seller` |
| T7 | Ghosting buyer locks seller funds | claim_after_timeout past window | `timeout_claim_success_after_warp`, `claim before deadline fails` (TS) |
| T8 | Admin pauses to trap funds / rug | Pause blocks create/fund only, never exits | `pause blocks create/fund but never exits` (TS), `pause_matrix_and_fee_edges` |
| T9 | Arbiter steals via bad split | split must sum exactly, fee on seller leg only | `arbiter resolves 60/40` (TS), `dispute_split_boundaries` |
| T10 | Rent grief / unclosed accounts | close only when Completed/Cancelled + vault empty, rent to buyer | `blocks cancel after work starts` (TS), `non-admin cannot pause; close gated` (TS), fuzz close attempts |
| T11 | Seller vanishes mid-escrow (funds locked in Pending) | reclaim_stale_milestone after seller deadline | `reclaim_fails_before_deadline_succeeds_after`, `reclaim_requires_active_milestone`, `full_abandonment_chain_leaves_nothing_locked`, `mixed_8_milestone_lifecycle_ends_empty_and_completed` |
| T12 | Arbiter vanishes after dispute (funds frozen in Disputed) | expire_dispute 50/50 after arbiter timeout | `expire_dispute_5050_with_odd_unit_to_buyer` |
| T13 | Arbiter colludes with buyer or seller | TRUST ASSUMPTION (no code mitigation; see Trust assumptions) | — |
| T14 | Compromised admin key rotates treasury/fee or lists malicious mint | Single-key admin is trusted; all updates emit events, open escrows unaffected (fee + treasury snapshots) | `admin can rotate fee/treasury and append mints` (TS), `treasury_rotation_only_affects_future_escrows` |
| T15 | Config realloc grief (allowlist growth) | Realloc capped at fixed max space, rent paid by admin signer, dup/full appends rejected | `admin can rotate…` duplicate case, unit `allowlist_bounds` |

## Out of scope

Front-end phishing, RPC censorship, USDC depeg, arbiter collusion off-chain.
Roadmap: multi-arbiter, reputation.

## Trust assumptions (explicit — see also AUDIT_PREP.md)

1. Arbiter acts honestly within a dispute; bounded by exact-sum splits and
   the 50/50 timeout expiry.
2. Admin key is uncompromised; it can pause create/fund and rotate
   fee/treasury/allowlist for future escrows only.
3. Upgrade authority (devnet deploy key) is trusted pre-mainnet.
4. `Clock::get()` is truthful for all windows and deadlines.
5. SPL Token classic program + mint issuers behave correctly.
