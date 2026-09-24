// Canonical PDA derivations. Seeds must match programs/escrowl/src/constants.rs:
// Config ["config"], Escrow ["escrow", buyer, seller, escrow_id_le],
// Vault ["vault", escrow].
import { PublicKey } from "@solana/web3.js";
import type { BN } from "@coral-xyz/anchor";

export const CONFIG_SEED = Buffer.from("config");
export const ESCROW_SEED = Buffer.from("escrow");
export const VAULT_SEED = Buffer.from("vault");

export function findConfigPda(programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([CONFIG_SEED], programId);
}

export function findEscrowPda(
  buyer: PublicKey,
  seller: PublicKey,
  escrowId: BN | bigint | number,
  programId: PublicKey
): [PublicKey, number] {
  const id = Buffer.alloc(8);
  const asBig = BigInt(escrowId.toString());
  id.writeBigUInt64LE(asBig);
  return PublicKey.findProgramAddressSync(
    [ESCROW_SEED, buyer.toBuffer(), seller.toBuffer(), id],
    programId
  );
}

export function findVaultPda(
  escrow: PublicKey,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [VAULT_SEED, escrow.toBuffer()],
    programId
  );
}
