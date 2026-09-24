# Escrowl demo script (3 minutes, seeded devnet)

Prereqs: `yarn seed-devnet` has run; `docs/DEPLOYMENT.md` has live addresses;
app running with `NEXT_PUBLIC_PROGRAM_ID` set. Figures below use the seed
defaults (250 bps fee, 0.5 + 0.3 test tokens).

## 0:00–0:30 — the problem + what Escrowl is (talking head / slides)

"Freelance and P2P payments fail on trust: buyers fear ghosting after
payment, sellers fear non-payment after delivery. Escrowl is a Solana
program that holds payment in a PDA vault and releases it per milestone —
with a designated arbiter for disputes. No custodian holds keys."

## 0:30–1:10 — create + fund (Dashboard → Create)

1. Connect wallet as buyer. Show Create form: seller, arbiter, two
   milestones (0.5 / 0.3), 24h review window.
2. Submit → two transactions (create, fund). Point at the vault balance on
   the Escrow Detail page: exactly 0.8, matching locked milestones (I1).
3. Show explorer link for the vault PDA.

## 1:10–1:50 — deliver + approve with fee split (Detail)

1. Switch to seller wallet → Submit milestone 0 with evidence hash.
2. Switch to buyer → Approve. Balances update live: seller +0.4875,
   treasury +0.0125 (2.5% fee, floor rounding, dust to seller — I5).
3. Milestone timeline flips to Released; attempting re-approve errors
   ("not in the right state" — I3, show the mapped message).

## 1:50–2:30 — dispute + exact-split resolve (Arbiter Console)

1. Seller submits milestone 1; buyer raises dispute within the window.
2. Arbiter console shows the disputed escrow. Enter an off-by-one split
   (e.g. 199,999 / 100,001) → transaction fails: splits must sum exactly
   (I4). Correct to 180,000 / 120,000 → resolves; escrow Completes.
3. Note the countdown: seller could have claimed after the window with no
   dispute (show the timer on a Submitted milestone).

## 2:30–2:50 — liveness exits (Detail page)

1. Show a stale Pending milestone past its seller deadline → Reclaim as
   buyer: the remainder refunds in one call (I11).
2. Show an expired dispute past the arbiter timeout → Expire as either
   party: exact 50/50 with the odd unit to the buyer (I11).

## 2:50–3:00 — close + security posture (slides)

1. Close the completed escrow; rent returns to buyer (I10).
2. Close-out slide: 10 invariants, threat model, 18 Rust tests + 30 TS
   integration cases, model fuzzing (~80k transitions, 1 spec finding
   fixed), `clippy -D warnings`, `cargo audit/deny`, verifiable build.
3. "Built for Colosseum Crypto World's Fair; applying to the Adevar Labs
   pre-audit track — the repo is structured to be audited."

## Recording tips

- Use devnet with seeded escrows so every state exists even if a live tx
  hiccups; rehearse wallet switching once.
- Keep explorer tabs pre-opened for vault + program addresses.
- If a transaction fails on camera, read the mapped error aloud — it
  demonstrates the guardrails working.
