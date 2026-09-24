// Friendly TypeScript mirrors of programs/escrowl/src/state + events.
// Field names match the IDL camelCase. BN is used for all u64/i64 wire values.
import { BN } from "@coral-xyz/anchor";
import type { PublicKey } from "@solana/web3.js";

export type MilestoneStatusKind =
  | "pending"
  | "submitted"
  | "released"
  | "disputed"
  | "resolved"
  | "refunded"
  | "cancelled";

export type EscrowStatusKind = "created" | "funded" | "completed" | "cancelled";

export type ReleaseReasonKind = "approved" | "timeout";

export interface Milestone {
  amount: BN;
  status: Record<MilestoneStatusKind, Record<string, never>> | MilestoneStatusKind;
  submittedAt: BN;
  terminalAt: BN;
  disputedAt: BN;
  evidenceHash: number[];
}

export interface ConfigAccount {
  admin: PublicKey;
  treasury: PublicKey;
  feeBps: number;
  paused: boolean;
  allowedMints: PublicKey[];
  bump: number;
}

export interface EscrowAccount {
  buyer: PublicKey;
  seller: PublicKey;
  arbiter: PublicKey;
  mint: PublicKey;
  vault: PublicKey;
  escrowId: BN;
  milestoneCount: number;
  milestones: Milestone[];
  totalAmount: BN;
  releasedAmount: BN;
  refundedAmount: BN;
  feeBpsSnapshot: number;
  treasurySnapshot: PublicKey;
  reviewWindowSecs: BN;
  sellerDeadlineSecs: BN;
  arbiterTimeoutSecs: BN;
  status: Record<EscrowStatusKind, Record<string, never>> | EscrowStatusKind;
  createdAt: BN;
  fundedAt: BN;
  bump: number;
  vaultBump: number;
}

export function milestoneStatus(m: Milestone): MilestoneStatusKind {
  if (typeof m.status === "string") return m.status;
  return Object.keys(m.status)[0] as MilestoneStatusKind;
}

export function escrowStatus(e: EscrowAccount): EscrowStatusKind {
  if (typeof e.status === "string") return e.status;
  return Object.keys(e.status)[0] as EscrowStatusKind;
}

/** Locked milestones back the vault balance (I1). */
export function lockedAmount(milestones: Milestone[]): BN {
  return milestones
    .filter((m) => ["pending", "submitted", "disputed"].includes(milestoneStatus(m)))
    .reduce((acc, m) => acc.add(m.amount), new BN(0));
}
