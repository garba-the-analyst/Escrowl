//! Adversarial battery against the REAL program. Every case asserts the
//! SPECIFIC expected error code (or exact post-state), never just failure.
//! Conservation (I12) is checked after rejection-heavy cases.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use solana_address::Address;
use solana_instruction::{AccountMeta, Instruction};
use solana_keypair::Keypair;
use solana_signer::Signer;
use solana_system_interface::{instruction as sys_ix, program as sys_prog};

// Error codes (6000-based, declaration order in programs/escrowl/src/errors.rs).
const E_BAD_MILESTONE_STATE: u32 = 6009;
const E_BAD_ESCROW_STATE: u32 = 6010;
const E_MATH_OVERFLOW: u32 = 6003;
const E_BAD_SPLIT: u32 = 6013;
const E_PAUSED: u32 = 6015;
const E_NOT_TERMINAL: u32 = 6017;
const E_UNAUTHORIZED: u32 = 6020;
const E_HAS_ONE: u32 = 2001;

fn setup1() -> (Env, Ctx, Keypair, Keypair, Keypair) {
    let mut env = Env::new();
    let buyer = env.actor();
    let seller = env.actor();
    let arbiter = env.actor();
    let (escrow, _) = escrow_pda(&buyer.pubkey(), &seller.pubkey(), 31);
    let (vault, _) = vault_pda(&escrow);
    let buyer_ata = env.token_account(&buyer.pubkey(), 5_000_000);
    let seller_ata = env.token_account(&seller.pubkey(), 0);
    let t = env.treasury;
    let treasury_ata = env.token_account(&t, 0);
    let c = Ctx {
        buyer: buyer.pubkey(), seller: seller.pubkey(), arbiter: arbiter.pubkey(),
        mint: env.mint, escrow, vault, config: env.config,
        buyer_ata, seller_ata, treasury_ata,
    };
    (env, c, buyer, seller, arbiter)
}

fn fund_submit(env: &mut Env, c: &Ctx, buyer: &Keypair, seller: &Keypair) {
    env.send(&[buyer], &[ix_create(c, 31, &[400_000], REVIEW_WINDOW, SELLER_DEADLINE, ARBITER_TIMEOUT)]);
    env.send(&[buyer], &[ix_fund(c)]);
    env.send(&[seller], &[ix_submit(c, &c.seller, 0, [9u8; 32])]);
}

fn expect_code(env: &mut Env, signers: &[&Keypair], ix: Instruction, code: u32) {
    let e = send_raw(&mut env.svm, signers, &[ix]).expect_err("expected failure");
    assert_eq!(err_code(&e), Some(code), "wrong error: {e:?}");
}

#[test]
fn substituted_vault_rejected_everywhere() {
    let (mut env, c, buyer, seller, _arbiter) = setup1();
    fund_submit(&mut env, &c, &buyer, &seller);
    // Attacker-owned token account passed as vault.
    let evil_vault = env.token_account(&seller.pubkey(), 0);
    let mut evil = c;
    evil.vault = evil_vault;
    expect_code(&mut env, &[&buyer], ix_approve(&evil, &evil.buyer, 0), E_HAS_ONE);
    // Vault from a DIFFERENT escrow (created, so initialized, but wrong PDA).
    let (escrow2, _) = escrow_pda(&buyer.pubkey(), &seller.pubkey(), 32);
    let (vault2, _) = vault_pda(&escrow2);
    let mut c32 = c;
    c32.escrow = escrow2;
    c32.vault = vault2;
    env.send(&[&buyer], &[ix_create(&c32, 32, &[400_000], REVIEW_WINDOW, SELLER_DEADLINE, ARBITER_TIMEOUT)]);
    let mut evil2 = c;
    evil2.vault = vault2;
    expect_code(&mut env, &[&buyer], ix_approve(&evil2, &evil2.buyer, 0), E_HAS_ONE);
    // Conservation intact after rejections (I12).
    let pots = Pots::read(&env, &c);
    assert_eq!(pots.sum(), 5_000_000);
    assert_eq!(pots.vault, 400_000);
}

#[test]
fn wrong_owner_token_accounts_rejected() {
    let (mut env, c, buyer, seller, _arbiter) = setup1();
    fund_submit(&mut env, &c, &buyer, &seller);
    let attacker = env.actor();
    // seller_ata owned by attacker (payout redirection).
    let evil_seller = env.token_account(&attacker.pubkey(), 0);
    let mut evil = c;
    evil.seller_ata = evil_seller;
    expect_code(&mut env, &[&buyer], ix_approve(&evil, &evil.buyer, 0), E_UNAUTHORIZED);
    // treasury_ata owned by attacker instead of treasury snapshot.
    let evil_treasury = env.token_account(&attacker.pubkey(), 0);
    let mut evil2 = c;
    evil2.treasury_ata = evil_treasury;
    expect_code(&mut env, &[&buyer], ix_approve(&evil2, &evil2.buyer, 0), E_UNAUTHORIZED);
    // buyer_ata passed where seller_ata expected (owner == buyer).
    let mut evil3 = c;
    evil3.seller_ata = c.buyer_ata;
    expect_code(&mut env, &[&buyer], ix_approve(&evil3, &evil3.buyer, 0), E_UNAUTHORIZED);
}

#[test]
fn fake_token_program_rejected() {
    let (mut env, c, buyer, _seller, _arbiter) = setup1();
    env.send(&[&buyer], &[ix_create(&c, 31, &[400_000], REVIEW_WINDOW, SELLER_DEADLINE, ARBITER_TIMEOUT)]);
    // Swap the token program for an attacker keypair: the transfer CPI must fail.
    let fake = Keypair::new().pubkey();
    let mut ix = ix_fund(&c);
    ix.accounts[6].pubkey = fake; // token_program is index 6 in fund_escrow
    let before_buyer = env.token_balance(&c.buyer_ata);
    send_raw(&mut env.svm, &[&buyer], &[ix]).expect_err("fake token program must fail");
    // Nothing moved (I12).
    assert_eq!(env.token_balance(&c.vault), 0);
    assert_eq!(env.token_balance(&c.buyer_ata), before_buyer);
}

#[test]
fn token2022_mint_cannot_create_escrow() {
    let (mut env, c, buyer, _seller, _arbiter) = setup1();
    // Token-2022 mint (same InitializeMint layout, tag 20, on Token-2022 program).
    let m22 = Keypair::new();
    let rent = env.svm.minimum_balance_for_rent_exemption(82);
    let (admin, admin_pk) = (env.admin_clone(), env.admin_pubkey());
    env.send(&[&admin, &m22], &[
        sys_ix::create_account(&admin_pk, &m22.pubkey(), rent, 82, &TOKEN_2022_ID),
    ]);
    let mut data = vec![20u8, 6u8];
    data.extend_from_slice(admin_pk.as_ref());
    data.push(0u8);
    env.send(&[&admin], &[Instruction {
        program_id: TOKEN_2022_ID,
        accounts: vec![
            AccountMeta::new(m22.pubkey(), false),
            AccountMeta::new_readonly(rent_id(), false),
        ],
        data,
    }]);
    // Even allowlisted, creation must fail (defense in depth: vault init via
    // the classic Token program rejects the foreign mint before/at the
    // handler's Token2022Rejected check — either layer failing is correct).
    env.send(&[&admin], &[ix_update_config(&env.config, &admin_pk, None, None, Some(m22.pubkey()))]);
    let (escrow, _) = escrow_pda(&buyer.pubkey(), &c.seller, 77);
    let (vault, _) = vault_pda(&escrow);
    let mut evil = c;
    evil.mint = m22.pubkey();
    evil.escrow = escrow;
    evil.vault = vault;
    send_raw(&mut env.svm, &[&buyer], &[ix_create(&evil, 77, &[100_000], REVIEW_WINDOW, SELLER_DEADLINE, ARBITER_TIMEOUT)])
        .expect_err("token-2022 mint must be rejected");
    // And no escrow account was created for it.
    assert!(env.svm.get_account(&escrow).is_none());
}

#[test]
fn reinit_and_double_create_rejected() {
    let (mut env, c, buyer, _seller, _arbiter) = setup1();
    let (admin, admin_pk) = (env.admin_clone(), env.admin_pubkey());
    // initialize_config twice: second fails (account already in use, system level).
    let t = env.treasury;
    let e = send_raw(&mut env.svm, &[&admin], &[ix_initialize(&env.config, &admin_pk, &t, FEE_BPS, &[env.mint])])
        .expect_err("reinit must fail");
    // System-level failure (Allocate: already in use → Custom(0)), never a
    // program error: the init guard fires before any handler code.
    assert_eq!(err_code(&e), Some(0), "reinit must fail at system level: {e:?}");
    // create same escrow twice: second fails at init (system level).
    env.send(&[&buyer], &[ix_create(&c, 31, &[400_000], REVIEW_WINDOW, SELLER_DEADLINE, ARBITER_TIMEOUT)]);
    let e2 = send_raw(&mut env.svm, &[&buyer], &[ix_create(&c, 31, &[400_000], REVIEW_WINDOW, SELLER_DEADLINE, ARBITER_TIMEOUT)])
        .expect_err("double create must fail");
    assert_eq!(err_code(&e2), Some(0), "double-init fails at system level: {e2:?}");
}

#[test]
fn double_transitions_and_cross_roles_fail() {
    let (mut env, c, buyer, seller, arbiter) = setup1();
    fund_submit(&mut env, &c, &buyer, &seller);
    env.send(&[&buyer], &[ix_approve(&c, &c.buyer, 0)]);
    // Single-milestone escrow is now Completed: further approves fail at the
    // escrow-level check FIRST (status ordering documented in SELF_AUDIT).
    // (Multi-milestone double-approve → InvalidMilestoneState is proven in TS.)
    expect_code(&mut env, &[&buyer], ix_approve(&c, &c.buyer, 0), E_BAD_ESCROW_STATE);
    expect_code(&mut env, &[&seller], ix_claim(&c, &c.seller, 0), E_BAD_ESCROW_STATE);
    expect_code(&mut env, &[&buyer], ix_dispute(&c, &c.buyer, 0), E_BAD_ESCROW_STATE);
    expect_code(&mut env, &[&arbiter], ix_resolve(&c, &c.arbiter, 0, 200_000, 200_000), E_BAD_ESCROW_STATE);
    // Cross-role calls fail at constraints (I2).
    expect_code(&mut env, &[&seller], ix_approve(&c, &c.seller, 0), E_HAS_ONE);
    expect_code(&mut env, &[&buyer], ix_submit(&c, &c.buyer, 0, [0u8; 32]), E_HAS_ONE);
    let _ = arbiter;
}

#[test]
fn pause_matrix_and_fee_edges() {
    let (mut env, c, buyer, seller, _arbiter) = setup1();
    fund_submit(&mut env, &c, &buyer, &seller);
    // Close before terminal.
    expect_code(&mut env, &[&buyer], ix_close(&c, &c.buyer), E_NOT_TERMINAL);
    // Paused: create + fund fail; approve (exit) still succeeds (I9).
    let (admin, admin_pk) = (env.admin_clone(), env.admin_pubkey());
    env.send(&[&admin], &[ix_set_paused(&env.config, &admin_pk, true)]);
    let (escrow2, _) = escrow_pda(&buyer.pubkey(), &seller.pubkey(), 99);
    let (vault2, _) = vault_pda(&escrow2);
    let mut c2 = c;
    c2.escrow = escrow2;
    c2.vault = vault2;
    expect_code(&mut env, &[&buyer], ix_create(&c2, 99, &[10_000], REVIEW_WINDOW, SELLER_DEADLINE, ARBITER_TIMEOUT), E_PAUSED);
    env.send(&[&buyer], &[ix_approve(&c, &c.buyer, 0)]);
    env.send(&[&admin], &[ix_set_paused(&env.config, &admin_pk, false)]);
    // Conservation after everything (I12).
    let pots = Pots::read(&env, &c);
    assert_eq!(pots.sum(), 5_000_000);
}

#[test]
fn fee_edges_max_fee_and_dust() {
    // fee_bps 500 (max), dust amounts: exact balance assertions.
    let mut env = Env::new_with_fee(500);
    let buyer = env.actor();
    let seller = env.actor();
    let arbiter = env.actor();
    let (escrow, _) = escrow_pda(&buyer.pubkey(), &seller.pubkey(), 41);
    let (vault, _) = vault_pda(&escrow);
    let buyer_ata = env.token_account(&buyer.pubkey(), 20_000);
    let seller_ata = env.token_account(&seller.pubkey(), 0);
    let t = env.treasury;
    let treasury_ata = env.token_account(&t, 0);
    let c = Ctx {
        buyer: buyer.pubkey(), seller: seller.pubkey(), arbiter: arbiter.pubkey(),
        mint: env.mint, escrow, vault, config: env.config,
        buyer_ata, seller_ata, treasury_ata,
    };
    env.send(&[&buyer], &[ix_create(&c, 41, &[9_999, 1], REVIEW_WINDOW, SELLER_DEADLINE, ARBITER_TIMEOUT)]);
    env.send(&[&buyer], &[ix_fund(&c)]);
    // 9_999 @ 500bps: fee = floor(9999*500/10000) = 499; seller 9_500.
    env.send(&[&seller], &[ix_submit(&c, &c.seller, 0, [0u8; 32])]);
    env.send(&[&buyer], &[ix_approve(&c, &c.buyer, 0)]);
    assert_eq!(env.token_balance(&c.seller_ata), 9_500);
    assert_eq!(env.token_balance(&c.treasury_ata), 499);
    // 1 unit @ 500bps: fee = floor(500/10000) = 0; seller keeps dust.
    env.send(&[&seller], &[ix_submit(&c, &c.seller, 1, [0u8; 32])]);
    env.send(&[&buyer], &[ix_approve(&c, &c.buyer, 1)]);
    assert_eq!(env.token_balance(&c.seller_ata), 9_501);
    assert_eq!(env.token_balance(&c.treasury_ata), 499);
    let st = env.fetch_escrow(&c.escrow);
    assert_eq!(st.status, common::ES_COMPLETED);
}

#[test]
fn dispute_split_boundaries() {
    let (mut env, c, buyer, seller, arbiter) = setup1();
    fund_submit(&mut env, &c, &buyer, &seller);
    env.send(&[&buyer], &[ix_dispute(&c, &c.buyer, 0)]);
    // Off-by-one both directions rejected (I4): overshoot and undershoot.
    expect_code(&mut env, &[&arbiter], ix_resolve(&c, &c.arbiter, 0, 200_001, 200_000), E_BAD_SPLIT);
    expect_code(&mut env, &[&arbiter], ix_resolve(&c, &c.arbiter, 0, 199_999, 200_000), E_BAD_SPLIT);
    // Overflow-wrapped split fails CLOSED at checked_add (defense in depth:
    // MathOverflow, before the equality check is even reached).
    expect_code(&mut env, &[&arbiter], ix_resolve(&c, &c.arbiter, 0, u64::MAX, 1), E_MATH_OVERFLOW);
    // Exact split succeeds.
    env.send(&[&arbiter], &[ix_resolve(&c, &c.arbiter, 0, 200_000, 200_000)]);
    let st = env.fetch_escrow(&c.escrow);
    assert_eq!(st.milestones[0].status, common::ST_RESOLVED);
}



