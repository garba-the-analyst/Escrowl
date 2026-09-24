// Happy-path lifecycle: config -> create -> fund -> submit/approve x2 -> close.
// Covers I1 (vault == locked), I5 (fee math), I7 (totals), I10 (close).
import * as anchor from "@coral-xyz/anchor";
import { SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { assert } from "chai";
import {
  FEE_BPS,
  REVIEW_WINDOW_SECS,
  SELLER_DEADLINE_SECS,
  ARBITER_TIMEOUT_SECS,
  ERR,
  airdrop,
  emptyAta,
  expectAnchorError,
  expectedFee,
  findConfigPda,
  findEscrowPda,
  findVaultPda,
  freshActors,
  fundedAta,
  setupMint,
  tokenBalance,
} from "../helpers";

describe("escrowl: happy path", () => {
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
  let escrowPda: anchor.web3.PublicKey;
  let vaultPda: anchor.web3.PublicKey;

  const AMOUNTS = [500_000n, 300_000n];
  const TOTAL = AMOUNTS[0] + AMOUNTS[1];
  const ESCROW_ID = new anchor.BN(1);

  it("initializes config (admin = provider wallet)", async () => {
    ({ buyer, seller, arbiter, treasuryOwner } = await freshActors(connection));
    await airdrop(connection, provider.wallet.publicKey, 5);
    mint = await setupMint(connection, buyer, buyer.publicKey);

    const [configPda] = findConfigPda(program.programId);
    await program.methods
      .initializeConfig(FEE_BPS, [mint])
      .accounts({
        admin: provider.wallet.publicKey,
        treasury: treasuryOwner.publicKey,
        config: configPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .rpc();

    const config = await program.account.config.fetch(configPda);
    assert.equal(config.feeBps, FEE_BPS);
    assert.isFalse(config.paused);
    assert.deepEqual(
      (config.allowedMints as anchor.web3.PublicKey[]).map((k) => k.toBase58()),
      [mint.toBase58()]
    );
  });

  it("buyer creates a 2-milestone escrow (I6/I7/I8)", async () => {
    [escrowPda] = findEscrowPda(
      buyer.publicKey,
      seller.publicKey,
      ESCROW_ID,
      program.programId
    );
    [vaultPda] = findVaultPda(escrowPda, program.programId);
    const [configPda] = findConfigPda(program.programId);

    await program.methods
      .createEscrow(
        ESCROW_ID,
        AMOUNTS.map((a) => new anchor.BN(a.toString())),
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
        escrow: escrowPda,
        vault: vaultPda,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .signers([buyer])
      .rpc();

    const escrow = await program.account.escrow.fetch(escrowPda);
    assert.equal(escrow.totalAmount.toString(), TOTAL.toString());
    assert.equal(escrow.milestoneCount, 2);
    assert.equal(escrow.feeBpsSnapshot, FEE_BPS);
    assert.deepEqual(escrow.status, { created: {} });
  });

  it("buyer funds the vault exactly once (I1)", async () => {
    const [configPda] = findConfigPda(program.programId);
    buyerAta = await fundedAta(
      connection,
      buyer,
      mint,
      buyer.publicKey,
      TOTAL + 100_000n // extra buffer stays with buyer
    );
    sellerAta = await emptyAta(connection, buyer, mint, seller.publicKey);
    treasuryAta = await emptyAta(
      connection,
      buyer,
      mint,
      treasuryOwner.publicKey
    );

    await program.methods
      .fundEscrow()
      .accounts({
        buyer: buyer.publicKey,
        escrow: escrowPda,
        vault: vaultPda,
        buyerAta,
        mint,
        config: configPda,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([buyer])
      .rpc();

    assert.equal((await tokenBalance(connection, vaultPda)).toString(), TOTAL.toString());
    const escrow = await program.account.escrow.fetch(escrowPda);
    assert.deepEqual(escrow.status, { funded: {} });

    // Double-fund must fail (I3: Created -> Funded only once).
    await expectAnchorError(
      program.methods
        .fundEscrow()
        .accounts({
          buyer: buyer.publicKey,
          escrow: escrowPda,
          vault: vaultPda,
          buyerAta,
          mint,
          config: configPda,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([buyer])
        .rpc(),
      ERR.InvalidEscrowState
    );
  });

  it("milestone 0: submit -> approve with exact fee split (I2/I5)", async () => {
    const [configPda] = findConfigPda(program.programId);
    const evidence = Array.from(Buffer.alloc(32, 1));

    await program.methods
      .submitMilestone(0, evidence)
      .accounts({ seller: seller.publicKey, escrow: escrowPda })
      .signers([seller])
      .rpc();

    const sellerBefore = await tokenBalance(connection, sellerAta);
    const treasuryBefore = await tokenBalance(connection, treasuryAta);

    await program.methods
      .approveMilestone(0)
      .accounts({
        buyer: buyer.publicKey,
        escrow: escrowPda,
        vault: vaultPda,
        sellerAta,
        treasuryAta,
        mint,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([buyer])
      .rpc();

    const fee = expectedFee(AMOUNTS[0], FEE_BPS);
    assert.equal(
      ((await tokenBalance(connection, sellerAta)) - sellerBefore).toString(),
      (AMOUNTS[0] - fee).toString()
    );
    assert.equal(
      ((await tokenBalance(connection, treasuryAta)) - treasuryBefore).toString(),
      fee.toString()
    );
    // Vault now backs only milestone 1 (I1).
    assert.equal(
      (await tokenBalance(connection, vaultPda)).toString(),
      AMOUNTS[1].toString()
    );
    const escrow = await program.account.escrow.fetch(escrowPda);
    assert.equal(escrow.releasedAmount.toString(), AMOUNTS[0].toString());

    // Double-approve must fail (I3: no double payout).
    await expectAnchorError(
      program.methods
        .approveMilestone(0)
        .accounts({
          buyer: buyer.publicKey,
          escrow: escrowPda,
          vault: vaultPda,
          sellerAta,
          treasuryAta,
          mint,
          tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
        })
        .signers([buyer])
        .rpc(),
      ERR.InvalidMilestoneState
    );
  });

  it("milestone 1 completes the escrow, then close reclaims rent (I10)", async () => {
    const [configPda] = findConfigPda(program.programId);
    await program.methods
      .submitMilestone(1, Array.from(Buffer.alloc(32, 2)))
      .accounts({ seller: seller.publicKey, escrow: escrowPda })
      .signers([seller])
      .rpc();
    await program.methods
      .approveMilestone(1)
      .accounts({
        buyer: buyer.publicKey,
        escrow: escrowPda,
        vault: vaultPda,
        sellerAta,
        treasuryAta,
        mint,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([buyer])
      .rpc();

    const escrow = await program.account.escrow.fetch(escrowPda);
    assert.deepEqual(escrow.status, { completed: {} });
    assert.equal((await tokenBalance(connection, vaultPda)).toString(), "0");

    await program.methods
      .closeEscrow()
      .accounts({
        buyer: buyer.publicKey,
        escrow: escrowPda,
        vault: vaultPda,
        tokenProgram: anchor.utils.token.TOKEN_PROGRAM_ID,
      })
      .signers([buyer])
      .rpc();

    // Escrow account closed — fetch must throw.
    try {
      await program.account.escrow.fetch(escrowPda);
      assert.fail("escrow account should be closed");
    } catch (e: any) {
      assert.match(e.message, /Account does not exist|failed to deserialize/i);
    }
  });
});
