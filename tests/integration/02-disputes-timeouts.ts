// Disputes + timeout edges: raise/resolve splits, window boundaries.
// Covers I2 (arbiter-only), I4 (exact splits), WindowNotElapsed/WindowElapsed.
// NOTE: localnet clock follows wall time and MIN window is 1h, so the
// *successful* timeout-claim path is covered by LiteSVM warp tests (Phase 4);
// here we assert the security-critical directions (early claim rejected,
// late dispute rejected is time-dependent and marked accordingly).
import * as anchor from "@coral-xyz/anchor";
import { SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { assert } from "chai";
import {
  FEE_BPS,
  REVIEW_WINDOW_SECS,
  SELLER_DEADLINE_SECS,
  ARBITER_TIMEOUT_SECS,
  ERR,
  emptyAta,
  expectAnchorError,
  findConfigPda,
  findEscrowPda,
  findVaultPda,
  freshActors,
  fundedAta,
  setupMint,
} from "../helpers";

describe("escrowl: disputes + timeouts", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Escrowl;
  const connection = provider.connection;

  let mint: anchor.web3.PublicKey;
  let buyer: anchor.web3.Keypair;
  let seller: anchor.web3.Keypair;
  let arbiter: anchor.web3.Keypair;
  let treasuryOwner: anchor.web3.Keypair;
  let buyerAta: anchor.web3.PublicKey;
  let sellerAta: anchor.web3.PublicKey;
  let treasuryAta: anchor.web3.PublicKey;
  let buyerRefundAta: anchor.web3.PublicKey;
  let escrowPda: anchor.web3.PublicKey;
  let vaultPda: anchor.web3.PublicKey;
  let configPda: anchor.web3.PublicKey;
  // Treasury owner is whoever initialized the singleton config (suite 01 on a
  // full run). All treasury-bound ATAs must use THAT pubkey (I2/Ux check).
  let treasuryPk: anchor.web3.PublicKey;

  const AMOUNT = 1_000_000n;

  async function newFundedEscrow(id: number) {
    const escrowId = new anchor.BN(id);
    const [e] = findEscrowPda(
      buyer.publicKey,
      seller.publicKey,
      escrowId,
      program.programId
    );
    const [v] = findVaultPda(e, program.programId);
    await program.methods
      .createEscrow(
        escrowId,
        [new anchor.BN(AMOUNT.toString())],
        new anchor.BN(REVIEW_WINDOW_SECS),
        new anchor.BN(SELLER_DEADLINE_SECS),
        new anchor.BN(ARBITER_TIMEOUT_SECS)
      )
      .accounts({
        buyer: buyer.publicKey,
        seller: seller.publicKey,
        arbiter: arbiter.publicKey,
        mint,
        config: configPda,
        escrow: e,
        vault: v,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([buyer])
      .rpc();
    await program.methods
      .fundEscrow()
      .accounts({
        buyer: buyer.publicKey,
        escrow: e,
        vault: v,
        buyerAta,
        mint,
        config: configPda,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([buyer])
      .rpc();
    await program.methods
      .submitMilestone(0, Array.from(Buffer.alloc(32, 7)))
      .accounts({ seller: seller.publicKey, escrow: e })
      .signers([seller])
      .rpc();
    return { e, v };
  }

  before(async () => {
    ({ buyer, seller, arbiter, treasuryOwner } = await freshActors(connection));
    mint = await setupMint(connection, buyer, buyer.publicKey);
    [configPda] = findConfigPda(program.programId);
    // Config is a singleton shared across suites: onboard this suite's mint
    // via admin update_config (each suite mints its own tokens, I8).
    let configExists = true;
    try {
      await program.account.config.fetch(configPda);
    } catch {
      configExists = false;
    }
    if (!configExists) {
      await program.methods
        .initializeConfig(FEE_BPS, [mint])
        .accounts({
          admin: provider.wallet.publicKey,
          treasury: treasuryOwner.publicKey,
          config: configPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
    } else {
      await program.methods
        .updateConfig(null, null, mint)
        .accounts({
          admin: provider.wallet.publicKey,
          config: configPda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .rpc();
    }
    buyerAta = await fundedAta(connection, buyer, mint, buyer.publicKey, 10_000_000n);
    sellerAta = await emptyAta(connection, buyer, mint, seller.publicKey);
    {
      const cfg = (await program.account.config.fetch(configPda)) as unknown as {
        treasury: anchor.web3.PublicKey;
      };
      treasuryPk = cfg.treasury;
    }
    treasuryAta = await emptyAta(connection, buyer, mint, treasuryPk);
    buyerRefundAta = await emptyAta(connection, buyer, mint, buyer.publicKey);
  });

  it("claim before deadline fails (protects buyer review window)", async () => {
    ({ e: escrowPda, v: vaultPda } = await newFundedEscrow(11));
    await expectAnchorError(
      program.methods
        .claimAfterTimeout(0)
        .accounts({
          seller: seller.publicKey,
          escrow: escrowPda,
          vault: vaultPda,
          sellerAta,
          treasuryAta,
          mint,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([seller])
        .rpc(),
      ERR.WindowNotElapsed
    );
  });

  it("buyer disputes in-window; non-buyer cannot (I2)", async () => {
    ({ e: escrowPda, v: vaultPda } = await newFundedEscrow(12));
    await program.methods
      .raiseDispute(0)
      .accounts({ buyer: buyer.publicKey, escrow: escrowPda })
      .signers([buyer])
      .rpc();
    const escrow = await program.account.escrow.fetch(escrowPda);
    assert.deepEqual(escrow.milestones[0].status, { disputed: {} });

    // Seller attempting to raise with the wrong signer fails at the
    // has_one constraint — before any state check (stronger guarantee, I2).
    await expectAnchorError(
      program.methods
        .raiseDispute(0)
        .accounts({ buyer: seller.publicKey, escrow: escrowPda })
        .signers([seller])
        .rpc(),
      ERR.HasOneConstraintViolated
    );
  });

  it("arbiter resolves 60/40; wrong splits rejected (I4)", async () => {
    ({ e: escrowPda, v: vaultPda } = await newFundedEscrow(13));
    await program.methods
      .raiseDispute(0)
      .accounts({ buyer: buyer.publicKey, escrow: escrowPda })
      .signers([buyer])
      .rpc();

    const base = {
      arbiter: arbiter.publicKey,
      escrow: escrowPda,
      vault: vaultPda,
      sellerAta,
      buyerAta: buyerRefundAta,
      treasuryAta,
      mint,
      tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
    };
    // Off-by-one split must fail exactly.
    await expectAnchorError(
      program.methods
        .resolveDispute(0, new anchor.BN("600000"), new anchor.BN("399999"))
        .accounts(base)
        .signers([arbiter])
        .rpc(),
      ERR.InvalidSplitSum
    );
    // Overshoot must also fail.
    await expectAnchorError(
      program.methods
        .resolveDispute(0, new anchor.BN("600001"), new anchor.BN("400000"))
        .accounts(base)
        .signers([arbiter])
        .rpc(),
      ERR.InvalidSplitSum
    );

    await program.methods
      .resolveDispute(0, new anchor.BN("600000"), new anchor.BN("400000"))
      .accounts(base)
      .signers([arbiter])
      .rpc();
    const escrow = await program.account.escrow.fetch(escrowPda);
    assert.deepEqual(escrow.milestones[0].status, { resolved: {} });
    assert.deepEqual(escrow.status, { completed: {} });
    assert.equal(escrow.releasedAmount.toString(), "600000");
    assert.equal(escrow.refundedAmount.toString(), "400000");
  });

  it("non-arbiter cannot resolve (I2: has_one constraint)", async () => {
    ({ e: escrowPda, v: vaultPda } = await newFundedEscrow(14));
    await program.methods
      .raiseDispute(0)
      .accounts({ buyer: buyer.publicKey, escrow: escrowPda })
      .signers([buyer])
      .rpc();
    await expectAnchorError(
      program.methods
        .resolveDispute(0, new anchor.BN("500000"), new anchor.BN("500000"))
        .accounts({
          arbiter: buyer.publicKey, // wrong signer
          escrow: escrowPda,
          vault: vaultPda,
          sellerAta,
          buyerAta: buyerRefundAta,
          treasuryAta,
          mint,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([buyer])
        .rpc(),
      ERR.HasOneConstraintViolated
    );
  });

  it("full-buyer refund path (100% to buyer, fee-free)", async () => {
    ({ e: escrowPda, v: vaultPda } = await newFundedEscrow(15));
    await program.methods
      .raiseDispute(0)
      .accounts({ buyer: buyer.publicKey, escrow: escrowPda })
      .signers([buyer])
      .rpc();
    await program.methods
      .resolveDispute(0, new anchor.BN("0"), new anchor.BN(AMOUNT.toString()))
      .accounts({
        arbiter: arbiter.publicKey,
        escrow: escrowPda,
        vault: vaultPda,
        sellerAta,
        buyerAta: buyerRefundAta,
        treasuryAta,
        mint,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([arbiter])
      .rpc();
    const escrow = await program.account.escrow.fetch(escrowPda);
    assert.equal(escrow.refundedAmount.toString(), AMOUNT.toString());
    assert.equal(escrow.releasedAmount.toString(), "0");
  });
});
