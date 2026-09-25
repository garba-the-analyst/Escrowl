// Seed devnet: mint + ATAs + one escrow in every state.
// Run: yarn seed-devnet   (or: ts-node scripts/seed-devnet.ts)
// Requires: deployed program ID in Anchor.toml, funded ANCHOR_WALLET on devnet,
// env MINT (optional — creates a new test mint if unset),
// env ESCROW_ID_BASE (optional numeric base for escrow ids).
import * as anchor from "@coral-xyz/anchor";
import {
  createAssociatedTokenAccountIdempotent,
  createMint,
  mintTo,
} from "@solana/spl-token";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  SystemProgram,
  Transaction,
  sendAndConfirmTransaction,
} from "@solana/web3.js";
import * as fs from "fs";
import { EscrowlClient } from "../sdk/src";

const FEE_BPS = 250;
const WINDOW = 86_400;
const EVIDENCE = Array.from(Buffer.alloc(32, 9));
const AMOUNTS = [500_000n, 300_000n];
const TOTAL: bigint = AMOUNTS[0] + AMOUNTS[1];

async function airdrop(conn: anchor.web3.Connection, to: anchor.web3.PublicKey) {
  // Public faucet is unreliable — fund from the deployer wallet instead.
  const walletPath =
    process.env.ANCHOR_WALLET ?? `${process.env.HOME}/.config/solana/id.json`;
  const deployer = Keypair.fromSecretKey(
    Buffer.from(JSON.parse(fs.readFileSync(walletPath, "utf8")))
  );
  const ix = SystemProgram.transfer({
    fromPubkey: deployer.publicKey,
    toPubkey: to,
    lamports: Math.floor(0.5 * LAMPORTS_PER_SOL),
  });
  await sendAndConfirmTransaction(conn, new Transaction().add(ix), [deployer]);
}

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const client = new EscrowlClient(provider);
  const conn = provider.connection;

  const buyer = Keypair.generate();
  const seller = Keypair.generate();
  const arbiter = Keypair.generate();
  const treasuryOwner = Keypair.generate();
  for (const kp of [buyer, seller, arbiter, treasuryOwner]) {
    await airdrop(conn, kp.publicKey);
  }

  const mint = process.env.MINT
    ? new anchor.web3.PublicKey(process.env.MINT)
    : await createMint(conn, buyer, buyer.publicKey, null, 6);
  console.log("mint:", mint.toBase58());

  try {
    await client.getConfig();
    console.log("config: exists, onboarding mint + rotating treasury via update_config");
    await client
      .updateConfig(provider.wallet.publicKey, null, treasuryOwner.publicKey, mint)
      .rpc();
  } catch {
    await client
      .initializeConfig(provider.wallet.publicKey, treasuryOwner.publicKey, FEE_BPS, [mint])
      .rpc();
    console.log("config: initialized");
  }

  // ATAs (idempotent) for all parties.
  const buyerAta = await createAssociatedTokenAccountIdempotent(
    conn, buyer, mint, buyer.publicKey
  );
  const sellerAta = await createAssociatedTokenAccountIdempotent(
    conn, buyer, mint, seller.publicKey
  );
  const treasuryAta = await createAssociatedTokenAccountIdempotent(
    conn, buyer, mint, treasuryOwner.publicKey
  );
  // Four funded escrows below → top up 4× total (+1 spare).
  await mintTo(conn, buyer, mint, buyerAta, buyer.publicKey, TOTAL * 5n);

  const base = Number(process.env.ESCROW_ID_BASE ?? (Date.now() % 100_000));
  const rows: [string, string][] = [];
  const link = (a: string) => `https://explorer.solana.com/address/${a}?cluster=devnet`;

  async function makeFunded(id: number, these: bigint[]) {
    const escrow = client.escrowPda(buyer.publicKey, seller.publicKey, id);
    await client
      .createEscrow({
        buyer: buyer.publicKey, seller: seller.publicKey, arbiter: arbiter.publicKey,
        mint, escrowId: id, milestoneAmounts: these, reviewWindowSecs: WINDOW,
        sellerDeadlineSecs: 7 * 86_400, arbiterTimeoutSecs: 30 * 86_400,
      })
      .signers([buyer])
      .rpc();
    await client
      .fundEscrow(buyer.publicKey, escrow, buyerAta, mint)
      .signers([buyer])
      .rpc();
    return escrow;
  }

  // 1. Created (never funded).
  {
    const id = base + 1;
    const escrow = client.escrowPda(buyer.publicKey, seller.publicKey, id);
    await client
      .createEscrow({
        buyer: buyer.publicKey, seller: seller.publicKey, arbiter: arbiter.publicKey,
        mint, escrowId: id, milestoneAmounts: AMOUNTS, reviewWindowSecs: WINDOW,
        sellerDeadlineSecs: 7 * 86_400, arbiterTimeoutSecs: 30 * 86_400,
      })
      .signers([buyer])
      .rpc();
    rows.push(["Created", escrow.toBase58()]);
  }

  // 2. Funded.
  rows.push(["Funded", (await makeFunded(base + 2, AMOUNTS)).toBase58()]);

  // 3. Submitted.
  {
    const escrow = await makeFunded(base + 3, [AMOUNTS[0]]);
    await client.submitMilestone(seller.publicKey, escrow, 0, EVIDENCE).signers([seller]).rpc();
    rows.push(["Submitted", escrow.toBase58()]);
  }

  // 4. Disputed.
  {
    const escrow = await makeFunded(base + 4, [AMOUNTS[0]]);
    await client.submitMilestone(seller.publicKey, escrow, 0, EVIDENCE).signers([seller]).rpc();
    await client.raiseDispute(buyer.publicKey, escrow, 0).signers([buyer]).rpc();
    rows.push(["Disputed", escrow.toBase58()]);
  }

  // 5. Completed (all milestones approved; left open to show terminal state).
  {
    const escrow = await makeFunded(base + 5, AMOUNTS);
    for (const idx of [0, 1]) {
      await client.submitMilestone(seller.publicKey, escrow, idx, EVIDENCE).signers([seller]).rpc();
      await client
        .approveMilestone(buyer.publicKey, escrow, sellerAta, treasuryAta, mint, idx)
        .signers([buyer])
        .rpc();
    }
    rows.push(["Completed", escrow.toBase58()]);
  }

  console.log("\n=== devnet seed summary ===");
  console.log(`mint:      ${mint.toBase58()}`);
  console.log(`buyer:     ${buyer.publicKey.toBase58()}`);
  console.log(`seller:    ${seller.publicKey.toBase58()}`);
  console.log(`arbiter:   ${arbiter.publicKey.toBase58()}`);
  console.log(`treasury:  ${treasuryOwner.publicKey.toBase58()}`);
  for (const [state, addr] of rows) {
    console.log(`${state.padEnd(10)} ${addr}  ${link(addr)}`);
  }
  console.log("\nPaste these into docs/DEPLOYMENT.md.");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
