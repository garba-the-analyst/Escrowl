# Escrowl App

Next.js 15.1 + React 19 + Tailwind v4 + wallet-adapter frontend for the Escrowl
milestone escrow program.

## Dev

```bash
npm install   # also links @escrowl/sdk via file: dependency (nothing to publish)
cp .env.example .env.local   # then fill in values (or write NEXT_PUBLIC_PROGRAM_ID directly)
npm run dev   # if port 3000 is taken, use: npm run dev -- --port 3123
```

Pinned: `next@15.1.12` (15.0.0 has CVE-2025-66478 and rejects React 19 stable),
`react@19`. Verified with `npm run build`.

## Env (.env.local)

```bash
NEXT_PUBLIC_RPC=https://api.devnet.solana.com
NEXT_PUBLIC_PROGRAM_ID=<deployed program id>
```

## Pages

- `/` Dashboard — connect-gated; tabs As Buyer / As Seller / As Arbiter
  (reads via `EscrowlClient.listEscrows`).
- `/create` — create + fund an escrow (1–8 milestones, review window 1h–30d).
  Mint defaults to a devnet USDC placeholder; only allowlisted SPL classic
  mints are accepted on-chain.
- `/escrow/[address]` — detail: vault balance, role-aware milestone actions
  (buyer approve/dispute/cancel/close, seller submit/claim, arbiter resolve
  with exact-sum split), live review countdowns, event feed.
- `/arbiter` — console listing escrows where you are arbiter with at least one
  disputed milestone, plus quick resolve forms.

## Notes

- Amounts are handled as strings/BN in base units — never JS floats for token math.
- Errors are mapped through the SDK `humanError` helper.
- The SDK is consumed as `@escrowl/sdk` (local `file:` dependency, transpiled
  via `transpilePackages` in `next.config.mjs`) — no relative `../../sdk`
  imports; web3 values come from `@solana/web3.js`, never `@coral-xyz/anchor`
  (0.31 only re-exports `BN`/`Program`/`AnchorProvider` as values).
