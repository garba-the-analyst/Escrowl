// Read-only devnet status board: fetches the 5 seeded escrows + vaults.
import * as anchor from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";
import { getAccount } from "@solana/spl-token";
import { EscrowlClient } from "../sdk/src";
import { escrowStatus, milestoneStatus } from "../sdk/src/types";

const ADDRESSES: Record<string, string> = {
  Created: "9PNUqAk71WgM2Lk71Eh78Kuwe42kjhjgDE3DFLy1u1hx",
  Funded: "HKXGgD5au34qzKoRKX8cKRjc4mrcd5j4P5C6dj2KRBG9",
  Submitted: "8z5MtZPnw6szboDox7X8FwMXAxrvVvmTEXD33eaffczr",
  Disputed: "3FRYL78kVRLpW14guZdi2hQPpjo82U2vVPseU9gKYuD5",
  Completed: "Gj9G6ZSCU27JvSR8mDSpH2cBqXqDYNvn3cCsqg2gvyNC",
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
