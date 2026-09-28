// Read-only devnet status board: fetches the 5 seeded escrows + vaults.
import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { getAccount } from "@solana/spl-token";
import { EscrowlClient } from "../sdk/src";
import { escrowStatus, milestoneStatus } from "../sdk/src/types";

const ADDRESSES: Record<string, string> = {
  Created: "huZokHPQ7kF29sZZ1EiCkPvRCcfYvMunwSa1jhtiyTu",
  Funded: "FKTp1chn8GE76W9BRXqo4bJgkvG9tKKsUTmh8YakD93r",
  Submitted: "J588Ai9rKmiqn3PTdozUXK7mBXxTESgaUXYzwxMdA6zC",
  Disputed: "7UvwNb9wg8nKDzFFFR3xQQ1qKeKqVKjr5sTpAaZiF49H",
  Completed: "C8QvcBZSfYNrrhuV7NcadXk5tfEYFU2wmb6yB7pQPz3h",
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
