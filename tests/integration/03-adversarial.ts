// Adversarial: creation validation, role attacks, ordering, pause, cancel/close.
// Covers I2 (roles), I3 (single terminal transition), I6 (distinct),
// I7 (milestone shape), I8 (mint), I9 (pause), I10 (close gating).
import * as anchor from "@coral-xyz/anchor";
import { createMint } from "@solana/spl-token";
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

describe("escrowl: adversarial", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Escrowl;
  const connection = provider.connection;

  let mint: anchor.web3.PublicKey;
  let buyer: anchor.web3.Keypair;
  let seller: anchor.web3.Keypair;
  let arbiter: anchor.web3.Keypair;
  let treasuryOwner: anchor.web3.Keypair;
  let configPda: anchor.web3.PublicKey;
  // Treasury of the singleton config (suite 01 on full runs) — all
  // treasury-bound ATAs must use this pubkey.
  let treasuryPk: anchor.web3.PublicKey;

  // NOTE: intentionally sync — returns the pending rpc() promise alongside
  // derived PDAs. (An earlier async version made `.tx` resolve to undefined
  // and silently voided every negative assertion using this helper.)
  function createOnly(
    id: number,
    amounts: bigint[],
    sellerKey = seller.publicKey,
    arbiterKey = arbiter.publicKey,
    useMint = mint,
    window = REVIEW_WINDOW_SECS
  ) {
    const escrowId = new anchor.BN(id);
    const [e] = findEscrowPda(
      buyer.publicKey,
      sellerKey,
      escrowId,
      program.programId
    );
    const [v] = findVaultPda(e, program.programId);
    const tx = program.methods
      .createEscrow(
        escrowId,
        amounts.map((a) => new anchor.BN(a.toString())),
        new anchor.BN(window),
        new anchor.BN(SELLER_DEADLINE_SECS),
        new anchor.BN(ARBITER_TIMEOUT_SECS)
      )
      .accounts({
        buyer: buyer.publicKey,
        seller: sellerKey,
        arbiter: arbiterKey,
        mint: useMint,
        config: configPda,
        escrow: e,
        vault: v,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([buyer])
      .rpc();
    return { tx, e, v };
  }

  before(async () => {
    ({ buyer, seller, arbiter, treasuryOwner } = await freshActors(connection));
    mint = await setupMint(connection, buyer, buyer.publicKey);
    [configPda] = findConfigPda(program.programId);
    // Config singleton: onboard this suite's mint via admin update_config.
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
    // Guarantee unpaused entry state (prior suites must leave it so).
    await program.methods
      .setPaused(false)
      .accounts({ admin: provider.wallet.publicKey, config: configPda })
      .rpc();
    {
      const cfg = (await program.account.config.fetch(configPda)) as unknown as {
        treasury: anchor.web3.PublicKey;
      };
      treasuryPk = cfg.treasury;
    }
  });

  after(async () => {
    // Never leave the protocol paused for other suites/devnet.
    await program.methods
      .setPaused(false)
      .accounts({ admin: provider.wallet.publicKey, config: configPda })
      .rpc();
  });

  it("rejects duplicate roles (I6)", async () => {
    // buyer == seller
    await expectAnchorError(
      createOnly(21, [100_000n], buyer.publicKey).tx,
      ERR.DuplicateRole
    );
    // buyer == arbiter
    await expectAnchorError(
      createOnly(22, [100_000n], seller.publicKey, buyer.publicKey).tx,
      ERR.DuplicateRole
    );
    // seller == arbiter
    await expectAnchorError(
      createOnly(23, [100_000n], seller.publicKey, seller.publicKey).tx,
      ERR.DuplicateRole
    );
  });

  it("rejects malformed milestone sets (I7)", async () => {
    await expectAnchorError(createOnly(24, []).tx, ERR.InvalidMilestoneCount);
    await expectAnchorError(
      createOnly(25, [1n, 1n, 1n, 1n, 1n, 1n, 1n, 1n, 1n]).tx,
      ERR.InvalidMilestoneCount
    );
    await expectAnchorError(
      createOnly(26, [100_000n, 0n]).tx,
      ERR.ZeroMilestoneAmount
    );
  });

  it("rejects bad window and foreign mint (I8)", async () => {
    await expectAnchorError(createOnly(27, [100_000n], seller.publicKey, arbiter.publicKey, mint, 60).tx, ERR.InvalidReviewWindow);
    const foreignMint = await createMint(
      connection,
      buyer,
      buyer.publicKey,
      null,
      6
    );
    await expectAnchorError(
      createOnly(28, [100_000n], seller.publicKey, arbiter.publicKey, foreignMint).tx,
      ERR.MintNotAllowed
    );
  });

  it("rejects out-of-order submit and double submit (I3)", async () => {
    const { e } = await createOnly(29, [100_000n, 100_000n]).tx.then(
      async () => {
        const escrowId = new anchor.BN(29);
        const [ee] = findEscrowPda(
          buyer.publicKey,
          seller.publicKey,
          escrowId,
          program.programId
        );
        return { e: ee };
      }
    );
    // Fund it first (submit requires Funded).
    const buyerAta = await fundedAta(connection, buyer, mint, buyer.publicKey, 300_000n);
    const [v] = findVaultPda(e, program.programId);
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

    // Index 1 before index 0 -> OutOfOrderSubmit.
    await expectAnchorError(
      program.methods
        .submitMilestone(1, Array.from(Buffer.alloc(32, 9)))
        .accounts({ seller: seller.publicKey, escrow: e })
        .signers([seller])
        .rpc(),
      ERR.OutOfOrderSubmit
    );
    // Index 2 out of bounds.
    await expectAnchorError(
      program.methods
        .submitMilestone(2, Array.from(Buffer.alloc(32, 9)))
        .accounts({ seller: seller.publicKey, escrow: e })
        .signers([seller])
        .rpc(),
      ERR.InvalidMilestoneIndex
    );
    // Correct submit, then re-submit same index -> InvalidMilestoneState.
    await program.methods
      .submitMilestone(0, Array.from(Buffer.alloc(32, 9)))
      .accounts({ seller: seller.publicKey, escrow: e })
      .signers([seller])
      .rpc();
    await expectAnchorError(
      program.methods
        .submitMilestone(0, Array.from(Buffer.alloc(32, 9)))
        .accounts({ seller: seller.publicKey, escrow: e })
        .signers([seller])
        .rpc(),
      ERR.InvalidMilestoneState
    );
    // Buyer (wrong role) attempts submit -> has_one violation (I2).
    await expectAnchorError(
      program.methods
        .submitMilestone(1, Array.from(Buffer.alloc(32, 9)))
        .accounts({ seller: buyer.publicKey, escrow: e })
        .signers([buyer])
        .rpc(),
      ERR.HasOneConstraintViolated
    );
  });

  it("rejects substituted vault / token accounts (T4)", async () => {
    // Escrow 29 has m0 Submitted: an approve with a swapped vault must fail
    // at the has_one check before touching funds.
    const [e29] = findEscrowPda(
      buyer.publicKey,
      seller.publicKey,
      new anchor.BN(29),
      program.programId
    );
    const [configPd] = findConfigPda(program.programId);
    const fakeVault = await emptyAta(connection, buyer, mint, seller.publicKey);
    const sellerAta = await emptyAta(connection, buyer, mint, seller.publicKey);
    const treasuryAta = await emptyAta(connection, buyer, mint, treasuryPk);
    await expectAnchorError(
      program.methods
        .approveMilestone(0)
        .accounts({
          buyer: buyer.publicKey,
          escrow: e29,
          vault: fakeVault, // NOT the escrow's PDA vault
          sellerAta,
          treasuryAta,
          mint,
          config: configPd,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([buyer])
        .rpc(),
      ERR.HasOneConstraintViolated
    );
  });

  it("blocks cancel after work starts; allows cancel when pristine", async () => {
    // Escrow 29 above has a submission -> cancel must fail.
    const [e29] = findEscrowPda(
      buyer.publicKey,
      seller.publicKey,
      new anchor.BN(29),
      program.programId
    );
    const [v29] = findVaultPda(e29, program.programId);
    const buyerAta29 = await emptyAta(connection, buyer, mint, buyer.publicKey);
    await expectAnchorError(
      program.methods
        .cancelEscrow()
        .accounts({
          buyer: buyer.publicKey,
          escrow: e29,
          vault: v29,
          buyerAta: buyerAta29,
          mint,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([buyer])
        .rpc(),
      ERR.CancelNotAllowed
    );

    // Pristine funded escrow cancels cleanly.
    await createOnly(30, [50_000n]).tx;
    const [e] = findEscrowPda(
      buyer.publicKey,
      seller.publicKey,
      new anchor.BN(30),
      program.programId
    );
    const [v] = findVaultPda(e, program.programId);
    const buyerAta = await fundedAta(connection, buyer, mint, buyer.publicKey, 50_000n);
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
      .cancelEscrow()
      .accounts({
        buyer: buyer.publicKey,
        escrow: e,
        vault: v,
        buyerAta,
        mint,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([buyer])
      .rpc();
    const escrow = await program.account.escrow.fetch(e);
    assert.deepEqual(escrow.status, { cancelled: {} });
  });

  it("pause blocks create/fund but never exits (I9)", async () => {
    // Prepare: one funded+submitted escrow (exit target), one created escrow (fund target).
    const { e: exitE, v: exitV } = await (async () => {
      const id = new anchor.BN(31);
      const [ee] = findEscrowPda(buyer.publicKey, seller.publicKey, id, program.programId);
      const [vv] = findVaultPda(ee, program.programId);
      await createOnly(31, [80_000n]).tx;
      const ata = await fundedAta(connection, buyer, mint, buyer.publicKey, 200_000n);
      await program.methods
        .fundEscrow()
        .accounts({
          buyer: buyer.publicKey, escrow: ee, vault: vv, buyerAta: ata,
          mint, config: configPda, tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([buyer])
        .rpc();
      await program.methods
        .submitMilestone(0, Array.from(Buffer.alloc(32, 3)))
        .accounts({ seller: seller.publicKey, escrow: ee })
        .signers([seller])
        .rpc();
      return { e: ee, v: vv };
    })();
    const { e: fundE, v: fundV } = await (async () => {
      const id = new anchor.BN(32);
      const [ee] = findEscrowPda(buyer.publicKey, seller.publicKey, id, program.programId);
      const [vv] = findVaultPda(ee, program.programId);
      await createOnly(32, [70_000n]).tx;
      return { e: ee, v: vv };
    })();
    const fundAta = await fundedAta(connection, buyer, mint, buyer.publicKey, 70_000n);

    await program.methods
      .setPaused(true)
      .accounts({ admin: provider.wallet.publicKey, config: configPda })
      .rpc();

    // Create blocked...
    await expectAnchorError(createOnly(33, [10_000n]).tx, ERR.Paused);
    // ...fund blocked...
    await expectAnchorError(
      program.methods
        .fundEscrow()
        .accounts({
          buyer: buyer.publicKey, escrow: fundE, vault: fundV, buyerAta: fundAta,
          mint, config: configPda, tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([buyer])
        .rpc(),
      ERR.Paused
    );

    // ...but approve (exit) still succeeds while paused.
    const sellerAta = await emptyAta(connection, buyer, mint, seller.publicKey);
    const treasuryAta = await emptyAta(connection, buyer, mint, treasuryPk);
    await program.methods
      .approveMilestone(0)
      .accounts({
        buyer: buyer.publicKey, escrow: exitE, vault: exitV,
        sellerAta, treasuryAta, mint,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([buyer])
      .rpc();
    const exited = await program.account.escrow.fetch(exitE);
    assert.deepEqual(exited.status, { completed: {} });

    await program.methods
      .setPaused(false)
      .accounts({ admin: provider.wallet.publicKey, config: configPda })
      .rpc();
  });

  it("admin can rotate fee/treasury and append mints; dupes + outsiders rejected", async () => {
    const adminAccts = {
      admin: provider.wallet.publicKey,
      config: configPda,
      systemProgram: anchor.web3.SystemProgram.programId,
    };
    // Fee rotation within bounds; open escrows unaffected (snapshot tested on-chain by feeBpsSnapshot).
    await program.methods.updateConfig(300, null, null).accounts(adminAccts).rpc();
    let config = await program.account.config.fetch(configPda);
    assert.equal(config.feeBps, 300);
    // Out-of-bounds fee rejected.
    await expectAnchorError(
      program.methods.updateConfig(501, null, null).accounts(adminAccts).rpc(),
      ERR.FeeTooHigh
    );
    // Duplicate mint append rejected.
    await expectAnchorError(
      program.methods.updateConfig(null, null, mint).accounts(adminAccts).rpc(),
      ERR.AllowlistUpdateInvalid
    );
    // Non-admin rejected by has_one.
    await expectAnchorError(
      program.methods
        .updateConfig(100, null, null)
        .accounts({ ...adminAccts, admin: buyer.publicKey })
        .signers([buyer])
        .rpc(),
      ERR.HasOneConstraintViolated
    );
    // Restore suite fee baseline.
    await program.methods.updateConfig(FEE_BPS, null, null).accounts(adminAccts).rpc();
    config = await program.account.config.fetch(configPda);
    assert.equal(config.feeBps, FEE_BPS);
  });

  it("non-admin cannot pause; close gated on terminal (I10)", async () => {    await expectAnchorError(
      program.methods
        .setPaused(true)
        .accounts({ admin: buyer.publicKey, config: configPda })
        .signers([buyer])
        .rpc(),
      ERR.HasOneConstraintViolated
    );

    // Escrow 32 is Created (never funded) — close must fail.
    const [e32] = findEscrowPda(
      buyer.publicKey,
      seller.publicKey,
      new anchor.BN(32),
      program.programId
    );
    const [v32] = findVaultPda(e32, program.programId);
    await expectAnchorError(
      program.methods
        .closeEscrow()
        .accounts({
          buyer: buyer.publicKey,
          escrow: e32,
          vault: v32,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([buyer])
        .rpc(),
      ERR.NotAllTerminal
    );
  });
});
