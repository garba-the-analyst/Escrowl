# Escrowl — Threat Model (STRIDE)

## Actors / Assets

Actors: buyer, seller, arbiter (pairwise distinct), admin, attacker.
Assets: vault SPL funds, escrow terms, fee revenue, rent.

## Threats

| # | Attack | Mitigation | Test proving it |
|---|---|---|---|
| T1 | Buyer approves twice / replay -> double payout | Terminal-once transition, status check | double-release negative test |
| T2 | Seller submits out of order / fake index | In-order check, index bounds | out-of-order test |
| T3 | Non-buyer approves, non-seller submits, non-arbiter resolves | has_one + signer role checks | role attack tests |
| T4 | Fake vault / wrong mint / wrong owner PDA substitution | Re-derive PDA, check mint allowlist + Token program owner | substitution tests (wrong vault, foreign mint) |
| T5 | Token-2022 hook / freeze / transfer fee abuse | Reject Token-2022, classic only | wrong-mint test |
| T6 | Overflow fee / dust theft | checked_* math, floor fee, dust to seller | overflow/dust tests |
| T7 | Ghosting buyer locks seller funds | claim_after_timeout past window | timeout boundary tests |
| T8 | Admin pauses to trap funds / rug | Pause blocks create/fund only, never exits | pause behavior tests |
| T9 | Arbiter steals via bad split | split must sum exactly, fee on seller leg only | split-sum tests |
| T10 | Rent grief / unclosed accounts | close only when all terminal + vault empty, rent to buyer | close tests |
| T11 | Compromised admin key rotates treasury/fee or lists malicious mint | Single-key admin is trusted; all updates emit events, open escrows unaffected (fee snapshot, no retroactive changes) | admin rotation + non-admin rejection tests |
| T12 | Config realloc grief (allowlist growth) | Realloc capped at fixed max space, rent paid by admin signer, dup/full appends rejected | duplicate + bounds tests |

## Out of scope

Front-end phishing, RPC censorship, USDC depeg, arbiter collusion off-chain.
Roadmap: multi-arbiter, reputation.
