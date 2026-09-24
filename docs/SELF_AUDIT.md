# Self-Audit — Escrowl (author's adversarial review, 2026-09-24)

Method: re-read every handler top to bottom against the account tables and
fund-flow orderings below. Every claim names the test that proves it; test
names are LiteSVM (`tests-litesvm/tests/*.rs`) unless prefixed TS
(`tests/integration/*.ts`).

## 1. Account constraint table

Legend: S = signer, W = writable, P = PDA re-derived (seeds+bump), M =
mint-owner check, O = token-owner check. Anchor `Account<T>` additionally
enforces program ownership of the account (3007 otherwise — proven by
`account_aliasing_rejected` case 3).

### initialize_config — admin(S,W) | treasury | config(init P) | system

| Constraint | Prevents | Test |
|---|---|---|
| payer = admin (mut) | unpaid init / rent grief | framework-enforced |
| seeds ["config"] + stored bump | config spoofing | singleton PDA, `reinit_and_double_create_rejected` |
| fee ≤ 500, allowlist 1..=4 | fee rug at birth, unbounded realloc | `admin can rotate…` (TS), unit `fee_bounds`/`allowlist_bounds` |

### set_paused / update_config — admin(S) | config(mut, has_one admin, P)

| Constraint | Prevents | Test |
|---|---|---|
| has_one admin | non-admin pause/param changes | TS `non-admin cannot pause`, `admin can rotate…` outsider case |
| realloc capped at max_space, payer admin | realloc rent grief | `admin can rotate…` (append succeeds, dup/full rejected) |

### create_escrow — buyer(S,W) | seller | arbiter | mint | config | escrow(init P) | vault(init P, token mint/authority) | system | token | rent

| Constraint | Prevents | Test |
|---|---|---|
| distinct buyer/seller/arbiter (handler) | self-dealing roles | `rejects duplicate roles` (TS), LiteSVM fuzz |
| mint ∈ allowlist + owner == Token ID + token_program == Token ID | Token-2022 / fake mint | `rejects bad window and foreign mint` (TS), `token2022_mint_cannot_create_escrow` |
| 1..=8 amounts, each > 0, windows bounded | oversized accounts, instant timeouts | `rejects malformed…`, unit bounds |
| escrow PDA ["escrow",buyer,seller,id] | escrow spoofing / cross-pairing | `reinit_and_double_create_rejected` (second create fails) |
| vault PDA ["vault",escrow], authority = escrow | vault substitution at birth | PDA seeds enforced by init |

### fund_escrow — buyer(S) | escrow(has_one buyer/vault/mint) | vault(M,O=escrow) | buyer_ata(M,O=buyer) | mint | config | token

| Constraint | Prevents | Test |
|---|---|---|
| status Created + vault empty (handler) | double-fund | TS `funds exactly once`, `double_transitions…` |
| vault.owner == escrow key | funding attacker's vault | `substituted_vault_rejected_everywhere` |
| buyer_ata.owner == buyer | funding from someone else's account | framework + `wrong_owner_token_accounts_rejected` pattern |
| exact total transfer, then vault == total assert | partial funding / rounding leak | TS vault==total assert, I12 fuzz |

### submit_milestone — seller(S) | escrow(has_one seller)

In-order + Pending-only enforced in handler. Tests: TS `rejects out-of-order…`,
LiteSVM fuzz (random out-of-order attempts rejected, state hash unchanged).

### approve / claim — buyer|seller(S) | escrow(has_one role/vault/mint) | vault(M,O=escrow) | seller_ata(M,O=seller) | treasury_ata(M,O=snapshot) | mint | token

| Constraint | Prevents | Test |
|---|---|---|
| Submitted-only + terminal-once flip | double payout (I3) | `double_transitions…`, TS double-approve, fuzz hammering |
| treasury owner == snapshot (NOT live config) | fee redirection after rotation | `treasury_rotation_only_affects_future_escrows` |
| seller_ata owner == seller | payout redirection | `wrong_owner_token_accounts_rejected`, `account_aliasing_rejected` |
| claim additionally: now >= submitted_at + window | early seller theft | `claim before deadline fails` (TS), warp boundaries (LiteSVM) |

### raise_dispute — buyer(S) | escrow(has_one buyer)

Submitted-only + now < deadline. Tests: `buyer disputes in-window` (TS),
`fuzz_late_dispute_rejected` (model), LiteSVM warp boundaries.

### resolve_dispute — arbiter(S) | escrow(has_one arbiter/vault/mint) | vault | seller_ata | buyer_ata | treasury_ata(snapshot) | mint | token

Exact-sum checked_add gate (I4); overflow fails closed with MathOverflow
(proven by `dispute_split_boundaries` u64::MAX case). Fee on seller leg only.
Test: `arbiter resolves 60/40` (TS), `expire_dispute_5050…`, fuzz splits.

### cancel_escrow — buyer(S) | escrow(has_one buyer/vault/mint) | vault | buyer_ata | mint | token

Created (vault empty → Cancelled marks) or Funded + zero-submitted (full
refund → Refunded marks). Tests: TS cancel suite, LiteSVM fuzz cancel paths.

### close_escrow — buyer(S) | escrow(has_one buyer/vault, close=buyer) | vault | token

Single guard: status Completed|Cancelled (every terminal path sets status) +
vault == 0. Tests: TS close suite, LiteSVM fuzz close attempts.

### reclaim_stale_milestone — buyer(S) | escrow | vault | buyer_ata | mint | token

Pending + all-prior-terminal + now >= active_since + deadline; cascade
refunds idx..end. Tests: `reclaim_fails_before_deadline…`,
`reclaim_requires_active_milestone`, 8-milestone unwind, fuzz.

### expire_dispute — authority(S, buyer-or-seller) | buyer | seller | escrow | vault | seller_ata | buyer_ata | treasury_ata(snapshot) | mint | token

Disputed + now >= disputed_at + timeout; 50/50, odd unit to buyer, fee on
seller leg. Tests: `expire_dispute_5050…`, fuzz expiry paths. Arbiter is
deliberately NOT an accepted signer (this exit exists for arbiter absence).

## 2. Fund-flow ordering (check → effect → interaction)

Every payout follows: (1) validate status/roles/timing/windows, (2) compute
amounts with checked math, (3) CPI transfers with PDA signer, (4) flip state
to terminal + update released/refunded, (5) emit event. State flips AFTER
transfers — safe here because a failed CPI aborts the whole transaction
(atomicity: fuzz asserts rejected ixns leave state bit-identical). No
reentrancy surface: Token program CPI has no callback into us; no
`remaining_accounts` are used anywhere (no unchecked extra accounts).

| Instruction | Checks before | Transfers | State after |
|---|---|---|---|
| fund | paused, Created, vault empty | buyer→vault total | Funded, funded_at |
| approve/claim | Funded, Submitted, roles | vault→seller, vault→treasury | Released, terminal_at, released+=amount, maybe Completed |
| resolve/expire | Funded, Disputed, window, split math | vault→seller/treasury/buyer | Resolved, terminal_at, released/refunded += legs, maybe Completed |
| cancel(Funded) | Funded, zero-submitted, vault==total | vault→buyer total | Refunded marks, refunded+=total, Cancelled |
| reclaim | Funded, Pending, prior-terminal, deadline | vault→buyer sum | Refunded marks, refunded+=sum, maybe Completed |

## 3. Boundary races (analyzed + tested)

Review window `[submit, deadline)`: dispute requires `now < deadline`
(strict); claim requires `now >= deadline`. The two are mutually exclusive —
no slot admits both, no slot admits neither (exactly one of approve/claim/
dispute is available to the entitled party at any time: before deadline the
buyer may approve or dispute and the seller waits; at/after deadline the
seller may claim and the buyer may still approve — both pay the seller the
same amounts, so the race is payoff-neutral).

Approve-vs-dispute: both valid pre-deadline; whoever lands first wins and the
other fails on state (Submitted→Released vs Submitted→Disputed). No fund
risk: both paths conserve (I12 fuzz asserts every accepted op).

Reclaim-vs-submit: reclaim requires Pending; a seller submit in the same slot
ordering either lands first (milestone Submitted → reclaim fails state) or
after (reclaim already Refunded → submit fails state). Exactly one wins;
funds conserved either way.

## 4. Aliasing analysis (tested in `account_aliasing_rejected`)

| Alias | Verdict |
|---|---|
| seller_ata == treasury_ata | Rejected (owner can't equal both seller and snapshot treasury; roles distinct) |
| vault as seller_ata | Rejected (vault owner is escrow PDA ≠ seller) |
| escrow PDA as buyer_ata | Rejected at Anchor deserialization (3007, owner is program) before constraints |
| buyer_ata as seller_ata | Rejected (owner == buyer ≠ seller) |
| vault == buyer_ata | Rejected (owner mismatch both directions) |
| authority == arbiter on expire | Rejected (must be buyer or seller) |

## 5. Remaining issues

| ID | Severity | Issue | Status |
|---|---|---|---|
| S-01 | Info | `VaultNotEmpty` (6019) unreachable via valid transitions (status gating implies empty vault) | Accepted: defense-in-depth second barrier, documented |
| S-02 | Info | `close_escrow` relies on status (not re-derived all_terminal) | Accepted: every terminal path sets status; fuzz + TS assert the coupling |
| S-03 | Low | Single-key admin (pause, fee/treasury rotation, allowlist) | Accepted pre-mainnet: documented trust assumption; mainnet recommendation is multisig/timelock in AUDIT_PREP.md |
| S-04 | Low | Single arbiter per escrow (collusion with one side) | Accepted: chosen jointly at creation; bounded by exact-split + 50/50 expiry. Multi-arbiter is roadmap |
| S-05 | Info | i64 timestamps near i64::MAX with 90-day windows | Mitigated: all deadline math via checked_add → MathOverflow; windows bounded ≤ 90d |

No new Critical/High/Medium findings. Specifically looked for and did NOT
find: reentrancy (no callbacks, atomic CPIs), PDA collision (canonical seeds
+ stored bumps, verified by substitution tests), signer confusion (every
mutating ix has an explicit Signer checked against stored roles), rent
drain (close returns to buyer; vault closed via CPI), duplicate mutable
accounts (aliasing table above), front-running beyond the analyzed races.
