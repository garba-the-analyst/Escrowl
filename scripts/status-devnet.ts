// Read-only devnet status board: fetches the 5 seeded escrows + vaults.
import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { getAccount } from "@solana/spl-token";
import { EscrowlClient } from "../sdk/src";
import { escrowStatus, milestoneStatus } from "../sdk/src/types";

const ADDRESSES: Record<string, string> = {
  Created: "3agdqg4YC1P5YUx1GR39ewqXJPiRaULRtApSLgXWWZJn",
  Funded: "8raZpxUUq2GGG4NFU4w5kP4MzoseLkqjuQnH9WnvTpiH",
  Submitted: "AtiH1BbP1CjXrx5mZEARF1W6sYfZ4SkdWJ5xCAwL9VA6",
  Disputed: "5tftybXA663PvxfJNFM82drpnEARBb3sQHU5vEfeLEBW",
  Completed: "Ff94z2WhQB8psAoDoAaFpnDugUV5wLQkVuRVQE1bEwiE",
};

async function main() {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const client = new EscrowlClient(provider);
  for (const [label, addr] of Object.entries(ADDRESSES)) {
    const key = new PublicKey(addr);
    const e = await client.getEscrow(key);
    const vault = await getAccount(provider.connection, e.vault);
    const ms = e.milestones
      .map((m, i) => `#${i}:${milestoneStatus(m)}:${m.amount.toString()}`)
      .join(" ");
    console.log(
      `${label.padEnd(10)} status=${escrowStatus(e)} total=${e.totalAmount} ` +
        `released=${e.releasedAmount} refunded=${e.refundedAmount} vault=${vault.amount} milestones=[${ms}]`
    );
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
