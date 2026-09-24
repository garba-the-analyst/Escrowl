// Typed client for the Escrowl program. Built on the hand-authored IDL in
// sdk/idl/escrowl.json (discriminators = real sighashes; re-verify with
// `anchor build` on a dev machine — see sdk/idl/README).
import {
  AnchorProvider,
  BN,
  Program,
} from "@coral-xyz/anchor";
import type { Idl } from "@coral-xyz/anchor";
import {
  PublicKey,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import { TOKEN_PROGRAM_ID } from "@solana/spl-token";
import idlJson from "../idl/escrowl.json";
import { findConfigPda, findEscrowPda, findVaultPda } from "./pdas";
import type { ConfigAccount, EscrowAccount } from "./types";

// eslint-disable-next-line @typescript-eslint/no-explicit-any
type AnyProgram = Program<any>;

export interface RoleFilter {
  buyer?: PublicKey;
  seller?: PublicKey;
  arbiter?: PublicKey;
}

export class EscrowlClient {
  readonly program: AnyProgram;
  readonly programId: PublicKey;
  readonly configPda: PublicKey;

  constructor(provider: AnchorProvider, programId?: PublicKey) {
    this.program = new Program(
      idlJson as unknown as Idl,
      provider
    ) as unknown as AnyProgram;
    this.programId = programId ?? this.program.programId;
    [this.configPda] = findConfigPda(this.programId);
  }

  /** Untyped account namespace — IDL is JSON, so account maps are any. */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private get ns(): any {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return this.program.account as any;
  }

  /** Untyped method builders — same reason; wrapper signatures stay typed. */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  private get m(): any {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    return this.program.methods as any;
  }

  // ---------- admin ----------

  initializeConfig(admin: PublicKey, treasury: PublicKey, feeBps: number, allowedMints: PublicKey[]) {
    return this.m.initializeConfig(feeBps, allowedMints)
      .accounts({
        admin,
        treasury,
        config: this.configPda,
        systemProgram: SystemProgram.programId,
      });
  }

  setPaused(admin: PublicKey, paused: boolean) {
    return this.m.setPaused(paused)
      .accounts({ admin, config: this.configPda });
  }

  /** Admin maintenance: rotate fee/treasury, append a mint. Null = no change. */
  updateConfig(
    admin: PublicKey,
    newFeeBps: number | null,
    newTreasury: PublicKey | null,
    addMint: PublicKey | null
  ) {
    return this.m.updateConfig(newFeeBps, newTreasury, addMint)
      .accounts({
        admin,
        config: this.configPda,
        systemProgram: SystemProgram.programId,
      });
  }

  getConfig(): Promise<ConfigAccount> {
    return this.ns.config.fetch(this.configPda) as Promise<ConfigAccount>;
  }

  // ---------- escrow lifecycle ----------

  escrowPda(buyer: PublicKey, seller: PublicKey, escrowId: BN | bigint | number) {
    const [pda] = findEscrowPda(buyer, seller, escrowId, this.programId);
    return pda;
  }

  vaultPda(escrow: PublicKey) {
    const [pda] = findVaultPda(escrow, this.programId);
    return pda;
  }

  createEscrow(opts: {
    buyer: PublicKey;
    seller: PublicKey;
    arbiter: PublicKey;
    mint: PublicKey;
    escrowId: BN | bigint | number;
    milestoneAmounts: (BN | bigint | number)[];
    reviewWindowSecs: BN | bigint | number;
    sellerDeadlineSecs: BN | bigint | number;
    arbiterTimeoutSecs: BN | bigint | number;
  }) {
    const escrow = this.escrowPda(opts.buyer, opts.seller, opts.escrowId);
    const toBn = (v: BN | bigint | number) => (v instanceof BN ? v : new BN(v.toString()));
    return this.m.createEscrow(
        toBn(opts.escrowId),
        opts.milestoneAmounts.map(toBn),
        toBn(opts.reviewWindowSecs),
        toBn(opts.sellerDeadlineSecs),
        toBn(opts.arbiterTimeoutSecs)
      )
      .accounts({
        buyer: opts.buyer,
        seller: opts.seller,
        arbiter: opts.arbiter,
        mint: opts.mint,
        config: this.configPda,
        escrow,
        vault: this.vaultPda(escrow),
        systemProgram: SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      });
  }

  fundEscrow(buyer: PublicKey, escrow: PublicKey, buyerAta: PublicKey, mint: PublicKey) {
    return this.m.fundEscrow().accounts({
      buyer,
      escrow,
      vault: this.vaultPda(escrow),
      buyerAta,
      mint,
      config: this.configPda,
      tokenProgram: TOKEN_PROGRAM_ID,
    });
  }

  submitMilestone(seller: PublicKey, escrow: PublicKey, index: number, evidenceHash: number[]) {
    return this.m.submitMilestone(index, evidenceHash)
      .accounts({ seller, escrow });
  }

  approveMilestone(
    buyer: PublicKey,
    escrow: PublicKey,
    sellerAta: PublicKey,
    treasuryAta: PublicKey,
    mint: PublicKey,
    index: number
  ) {
    return this.m.approveMilestone(index).accounts({
      buyer,
      escrow,
      vault: this.vaultPda(escrow),
      sellerAta,
      treasuryAta,
      mint,
      tokenProgram: TOKEN_PROGRAM_ID,
    });
  }

  claimAfterTimeout(
    seller: PublicKey,
    escrow: PublicKey,
    sellerAta: PublicKey,
    treasuryAta: PublicKey,
    mint: PublicKey,
    index: number
  ) {
    return this.m.claimAfterTimeout(index).accounts({
      seller,
      escrow,
      vault: this.vaultPda(escrow),
      sellerAta,
      treasuryAta,
      mint,
      tokenProgram: TOKEN_PROGRAM_ID,
    });
  }

  raiseDispute(buyer: PublicKey, escrow: PublicKey, index: number) {
    return this.m.raiseDispute(index)
      .accounts({ buyer, escrow });
  }

  resolveDispute(opts: {
    arbiter: PublicKey;
    escrow: PublicKey;
    sellerAta: PublicKey;
    buyerAta: PublicKey;
    treasuryAta: PublicKey;
    mint: PublicKey;
    index: number;
    sellerAmount: BN | bigint | number;
    buyerAmount: BN | bigint | number;
  }) {
    const toBn = (v: BN | bigint | number) => (v instanceof BN ? v : new BN(v.toString()));
    return this.m.resolveDispute(opts.index, toBn(opts.sellerAmount), toBn(opts.buyerAmount))
      .accounts({
        arbiter: opts.arbiter,
        escrow: opts.escrow,
        vault: this.vaultPda(opts.escrow),
        sellerAta: opts.sellerAta,
        buyerAta: opts.buyerAta,
        treasuryAta: opts.treasuryAta,
        mint: opts.mint,
        tokenProgram: TOKEN_PROGRAM_ID,
      });
  }

  /** Buyer refunds a stale Pending milestone (+ later Pending) after the
   * seller deadline. Liveness exit (I11). */
  reclaimStaleMilestone(buyer: PublicKey, escrow: PublicKey, buyerAta: PublicKey, mint: PublicKey, index: number) {
    return this.m.reclaimStaleMilestone(index).accounts({
      buyer,
      escrow,
      vault: this.vaultPda(escrow),
      buyerAta,
      mint,
      tokenProgram: TOKEN_PROGRAM_ID,
    });
  }

  /** Either counterparty unlocks an abandoned dispute 50/50 after the
   * arbiter timeout. Liveness exit (I11). */
  expireDispute(
    authority: PublicKey,
    buyer: PublicKey,
    seller: PublicKey,
    escrow: PublicKey,
    sellerAta: PublicKey,
    buyerAta: PublicKey,
    treasuryAta: PublicKey,
    mint: PublicKey,
    index: number
  ) {
    return this.m.expireDispute(index).accounts({
      authority,
      buyer,
      seller,
      escrow,
      vault: this.vaultPda(escrow),
      sellerAta,
      buyerAta,
      treasuryAta,
      mint,
      tokenProgram: TOKEN_PROGRAM_ID,
    });
  }

  cancelEscrow(buyer: PublicKey, escrow: PublicKey, buyerAta: PublicKey, mint: PublicKey) {
    return this.m.cancelEscrow().accounts({
      buyer,
      escrow,
      vault: this.vaultPda(escrow),
      buyerAta,
      mint,
      tokenProgram: TOKEN_PROGRAM_ID,
    });
  }

  closeEscrow(buyer: PublicKey, escrow: PublicKey) {
    return this.m.closeEscrow().accounts({
      buyer,
      escrow,
      vault: this.vaultPda(escrow),
      tokenProgram: TOKEN_PROGRAM_ID,
    });
  }

  // ---------- reads ----------

  getEscrow(escrow: PublicKey): Promise<EscrowAccount> {
    return this.ns.escrow.fetch(escrow) as Promise<EscrowAccount>;
  }

  /** List escrows where `role` equals `wallet` (memcmp on fixed-offset pubkeys). */
  async listEscrows(role: "buyer" | "seller" | "arbiter", wallet: PublicKey) {
    const offset =
      role === "buyer" ? 8 : role === "seller" ? 8 + 32 : 8 + 64;
    return (await this.ns.escrow.all([
      { memcmp: { offset, bytes: wallet.toBase58() } },
    ])) as unknown as { publicKey: PublicKey; account: EscrowAccount }[];
  }

  // ---------- events ----------

  /** Subscribe to every state-change event; returns listener ids. */
  onAllEvents(cb: (name: string, data: unknown) => void): number[] {
    const names = [
      "ConfigInitialized",
      "ConfigUpdated",
      "PausedUpdated",
      "EscrowCreated",
      "EscrowFunded",
      "MilestoneSubmitted",
      "MilestoneReleased",
      "DisputeRaised",
      "DisputeResolved",
      "DisputeExpired",
      "MilestoneReclaimed",
      "EscrowCancelled",
      "EscrowClosed",
    ];
    return names.map((n) => this.program.addEventListener(n, (data) => cb(n, data)));
  }

  async removeListeners(ids: number[]): Promise<void> {
    await Promise.all(ids.map((id) => this.program.removeEventListener(id)));
  }
}

/** fee = floor(amount * bps / 10_000), dust to seller — mirrors on-chain math. */
export function calcFee(amount: bigint, feeBps: number): bigint {
  return (amount * BigInt(feeBps)) / 10_000n;
}
