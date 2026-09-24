// Full lifecycle example (devnet). Run: ts-node sdk/examples/lifecycle.ts
// Requires: funded buyer keypair, a devnet SPL mint, deployed program ID in
// Anchor.toml + sdk/idl/escrowl.json address field.
import { AnchorProvider, BN, Keypair, PublicKey } from "@coral-xyz/anchor";
import { getAssociatedTokenAddress } from "@solana/spl-token";
import { EscrowlClient, humanError } from "../src";

async function main() {
  const provider = AnchorProvider.env();
  const client = new EscrowlClient(provider);

  const buyer = provider.wallet.publicKey;
  const seller = Keypair.generate().publicKey;
  const arbiter = Keypair.generate().publicKey;
  const mint = new PublicKey(process.env.MINT!);
  const escrowId = Date.now() % 1_000_000;

  const escrow = client.escrowPda(buyer, seller, escrowId);
  console.log("escrow:", escrow.toBase58());

  const buyerAta = await getAssociatedTokenAddress(mint, buyer);
  const sellerAta = await getAssociatedTokenAddress(mint, seller);

  await client
    .createEscrow({
      buyer, seller, arbiter, mint,
      escrowId,
      milestoneAmounts: [new BN(500_000), new BN(300_000)],
      reviewWindowSecs: new BN(86_400),
      sellerDeadlineSecs: new BN(7 * 86_400),
      arbiterTimeoutSecs: new BN(30 * 86_400),
    })
    .rpc();
  console.log("created");

  await client.fundEscrow(buyer, escrow, buyerAta, mint).rpc();
  console.log("funded");

  const state = await client.getEscrow(escrow);
  console.log("total:", state.totalAmount.toString(), "status:", state.status);
}

main().catch((e) => {
  console.error(humanError(e));
  process.exit(1);
});
