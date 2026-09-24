# Escrowl SDK (`@escrowl/sdk`)

Typed client over `sdk/idl/escrowl.json` (hand-authored, sighash-verified —
see `sdk/idl/README.md`; regenerate with `anchor build` on a dev machine
and diff before shipping).

## Install

```bash
npm install @escrowl/sdk  # once published; for now: file:../sdk
```

## Usage

```ts
import { AnchorProvider } from "@coral-xyz/anchor";
import { BN } from "@coral-xyz/anchor";
import { EscrowlClient, humanError } from "@escrowl/sdk";

const client = new EscrowlClient(AnchorProvider.env());

// Buyer creates a 2-milestone escrow (amounts in base units, e.g. 6-decimal USDC).
await client
  .createEscrow({
    buyer, seller, arbiter, mint,
    escrowId: 1n,
    milestoneAmounts: [500_000n, 300_000n],
    reviewWindowSecs: 86_400n,
  })
  .rpc();

// Buyer funds the PDA vault exactly once.
await client.fundEscrow(buyer, escrow, buyerAta, mint).rpc();

// Liveness exits (buyer-only reclaim after seller deadline; either party
// expires an abandoned dispute 50/50 after the arbiter timeout).
await client.reclaimStaleMilestone(buyer, escrow, buyerAta, mint, 1).rpc();
await client.expireDispute(buyer, buyer, seller, escrow, sellerAta, buyerAta, treasuryAta, mint, 1).rpc();

// Seller submits with a 32-byte evidence hash (e.g. sha256 of deliverable).
await client.submitMilestone(seller, escrow, 0, [...evidenceHash]).rpc();

// Buyer approves — seller gets amount minus fee, treasury gets the fee.
await client.approveMilestone(buyer, escrow, sellerAta, treasuryAta, mint, 0).rpc();

// Timeout path (seller, after review window with no dispute).
await client.claimAfterTimeout(seller, escrow, sellerAta, treasuryAta, mint, 1).rpc();

// Dispute path (buyer, in-window; arbiter splits exactly).
await client.raiseDispute(buyer, escrow, 1).rpc();
await client
  .resolveDispute({
    arbiter, escrow, sellerAta, buyerAta, treasuryAta, mint,
    index: 1, sellerAmount: 600_000n, buyerAmount: 400_000n, // must sum exactly
  })
  .rpc();

// Reads.
const escrowState = await client.getEscrow(escrow);
const asSeller = await client.listEscrows("seller", seller);
const ids = client.onAllEvents((name, data) => console.log(name, data));

// Errors map to human copy for UI.
try {
  await client.approveMilestone(buyer, escrow, sellerAta, treasuryAta, mint, 0).rpc();
} catch (e) {
  console.error(humanError(e)); // "Milestone is not in the right state..."
}
```

## Modules

- `client.ts` — `EscrowlClient` (all 14 instructions incl. `reclaimStaleMilestone`
  and `expireDispute` liveness exits, fetchers, memcmp listing, event
  subscriptions) + `calcFee`
- `pdas.ts` — canonical PDA derivations (must match `constants.rs`)
- `errors.ts` — codes 6000–6024 + `humanError`/`isError` for UI copy
- `types.ts` — `ConfigAccount`, `EscrowAccount`, status helpers

See `sdk/examples/lifecycle.ts` for the full runnable flow.
