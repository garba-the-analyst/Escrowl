// Shared Anchor integration-test helpers.
// Every test maps to docs/INVARIANTS.md (I1-I10) — see per-file headers.
import * as anchor from "@coral-xyz/anchor";
import {
  createMint,
  createAssociatedTokenAccountIdempotent,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import {
  Keypair,
  LAMPORTS_PER_SOL,
  PublicKey,
  Connection,
} from "@solana/web3.js";
import { assert } from "chai";

export const FEE_BPS = 250; // 2.5%
export const REVIEW_WINDOW_SECS = 86_400; // 24h
export const MINT_DECIMALS = 6;

export const CONFIG_SEED = Buffer.from("config");
export const ESCROW_SEED = Buffer.from("escrow");
export const VAULT_SEED = Buffer.from("vault");

/// Anchor custom error names in declaration order (6000 + index).
export const ERR = {
  InvalidMilestoneCount: "InvalidMilestoneCount",
  ZeroMilestoneAmount: "ZeroMilestoneAmount",
  DuplicateRole: "DuplicateRole",
  MintNotAllowed: "MintNotAllowed",
  InvalidMilestoneIndex: "InvalidMilestoneIndex",
  OutOfOrderSubmit: "OutOfOrderSubmit",
  InvalidMilestoneState: "InvalidMilestoneState",
  InvalidEscrowState: "InvalidEscrowState",
  WindowNotElapsed: "WindowNotElapsed",
  WindowElapsed: "WindowElapsed",
  InvalidSplitSum: "InvalidSplitSum",
  FeeTooHigh: "FeeTooHigh",
  Paused: "Paused",
  CancelNotAllowed: "CancelNotAllowed",
  NotAllTerminal: "NotAllTerminal",
  Unauthorized: "Unauthorized",
  InvalidReviewWindow: "InvalidReviewWindow",
  AllowlistUpdateInvalid: "AllowlistUpdateInvalid",
  // Anchor framework constraint violations (has_one, signer, etc.)
  HasOneConstraintViolated: "HasOneConstraintViolated",
  ConstraintSigner: "ConstraintSigner",
} as const;

export function findConfigPda(programId: PublicKey): [PublicKey, number] {
  return PublicKey.findProgramAddressSync([CONFIG_SEED], programId);
}

export function findEscrowPda(
  buyer: PublicKey,
  seller: PublicKey,
  escrowId: anchor.BN,
  programId: PublicKey
): [PublicKey, number] {
  return PublicKey.findProgramAddressSync(
    [
      ESCROW_SEED,
      buyer.toBuffer(),
      seller.toBuffer(),
      escrowId.toArrayLike(Buffer, "le", 8),
    ],
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

/// Fund a keypair with SOL for tx fees + rent.
export async function airdrop(
  connection: Connection,
  to: PublicKey,
  sol = 2
): Promise<void> {
  const sig = await connection.requestAirdrop(to, sol * LAMPORTS_PER_SOL);
  await connection.confirmTransaction(sig, "confirmed");
}

/// Create an SPL mint owned by `authority`.
export async function setupMint(
  connection: Connection,
  payer: Keypair,
  authority: PublicKey
) {
  return createMint(connection, payer, authority, null, MINT_DECIMALS);
}

/// Create ATA for `owner` and mint `amount` to it.
/// Idempotent: suites reuse buyer ATAs across escrows (same owner+mint).
export async function fundedAta(
  connection: Connection,
  payer: Keypair,
  mint: PublicKey,
  owner: PublicKey,
  amount: number | bigint
): Promise<PublicKey> {
  const ata = await createAssociatedTokenAccountIdempotent(
    connection,
    payer,
    mint,
    owner
  );
  await mintTo(connection, payer, mint, ata, authority(payer), amount);
  return ata;
}

/// ATA without funding (for treasury/recipient accounts). Idempotent.
export async function emptyAta(
  connection: Connection,
  payer: Keypair,
  mint: PublicKey,
  owner: PublicKey
): Promise<PublicKey> {
  return createAssociatedTokenAccountIdempotent(connection, payer, mint, owner);
}

function authority(payer: Keypair): PublicKey {
  return payer.publicKey;
}

export async function tokenBalance(
  connection: Connection,
  ata: PublicKey
): Promise<bigint> {
  const acc = await getAccount(connection, ata);
  return acc.amount;
}

/** Assert a transaction fails with an Anchor error containing `code`.
 * Matches the program error NAME (errorCode.code), the numeric code, or any
 * message substring — Anchor surfaces all three shapes depending on context.
 */
export async function expectAnchorError(
  promise: Promise<unknown>,
  code: string
): Promise<void> {
  try {
    await promise;
  } catch (e: any) {
    const errObj = e?.error ?? {};
    const codeName: string | undefined = errObj?.errorCode?.code;
    const codeNumber: number | undefined = errObj?.errorCode?.number;
    const msg: string =
      errObj?.errorMessage ?? e?.message ?? JSON.stringify(e);
    const matched =
      codeName === code ||
      (codeNumber !== undefined && ERROR_NUMBERS[code] === codeNumber) ||
      msg.includes(code);
    assert(
      matched,
      `expected error "${code}", got code=${codeName}(${codeNumber}) msg=${msg}`
    );
    return;
  }
  assert.fail(`expected transaction to fail with "${code}" but it succeeded`);
}

/** Program error names → numeric codes (6000 + declaration order, errors.rs). */
export const ERROR_NUMBERS: Record<string, number> = {
  InvalidMilestoneCount: 6000,
  ZeroMilestoneAmount: 6001,
  TotalMismatch: 6002,
  MathOverflow: 6003,
  DuplicateRole: 6004,
  MintNotAllowed: 6005,
  Token2022Rejected: 6006,
  InvalidMilestoneIndex: 6007,
  OutOfOrderSubmit: 6008,
  InvalidMilestoneState: 6009,
  InvalidEscrowState: 6010,
  WindowNotElapsed: 6011,
  WindowElapsed: 6012,
  InvalidSplitSum: 6013,
  FeeTooHigh: 6014,
  Paused: 6015,
  CancelNotAllowed: 6016,
  NotAllTerminal: 6017,
  VaultBalanceMismatch: 6018,
  VaultNotEmpty: 6019,
  Unauthorized: 6020,
  InvalidReviewWindow: 6021,
  AllowlistUpdateInvalid: 6022,
  SellerDeadlineNotElapsed: 6023,
  DisputeNotExpired: 6024,
  HasOneConstraintViolated: 2001,
};

export const SELLER_DEADLINE_SECS = 86_400 * 7;
export const ARBITER_TIMEOUT_SECS = 86_400 * 30;

/** fee = floor(amount * bps / 10_000) — mirrors utils/math.rs (I5). */
export function expectedFee(amount: bigint, bps: number): bigint {
  return (amount * BigInt(bps)) / 10_000n;
}

/** Fresh funded buyer/seller/arbiter/treasury set. */
export async function freshActors(connection: Connection) {
  const buyer = Keypair.generate();
  const seller = Keypair.generate();
  const arbiter = Keypair.generate();
  const treasuryOwner = Keypair.generate();
  for (const kp of [buyer, seller, arbiter, treasuryOwner]) {
    await airdrop(connection, kp.publicKey);
  }
  return { buyer, seller, arbiter, treasuryOwner };
}
