//! Real-program lifecycle on LiteSVM: happy path, timeout-claim SUCCESS
//! (clock warp — unreachable on localnet), window boundaries, fee/treasury
//! snapshot rotation proof. Exact balance assertions everywhere.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use solana_signer::Signer;

fn setup(amounts: &[u64]) -> (Env, Ctx, solana_keypair::Keypair, solana_keypair::Keypair, solana_keypair::Keypair) {
    let mut env = Env::new();
    let buyer = env.actor();
    let seller = env.actor();
    let arbiter = env.actor();
    let (escrow, _) = escrow_pda(&buyer.pubkey(), &seller.pubkey(), 7);
    let (vault, _) = vault_pda(&escrow);
    let buyer_ata = env.token_account(&buyer.pubkey(), amounts.iter().sum::<u64>() + 1_000_000);
    let seller_ata = env.token_account(&seller.pubkey(), 0);
    let treasury = env.treasury;
    let treasury_ata = env.token_account(&treasury, 0);
    let c = Ctx {
        buyer: buyer.pubkey(),
        seller: seller.pubkey(),
        arbiter: arbiter.pubkey(),
        mint: env.mint,
        escrow,
        vault,
        config: env.config,
        buyer_ata,
        seller_ata,
        treasury_ata,
    };
    env.send(&[&buyer], &[ix_create(
        &c, 7, amounts, REVIEW_WINDOW, SELLER_DEADLINE, ARBITER_TIMEOUT,
    )]);
    (env, c, buyer, seller, arbiter)
}

#[test]
fn lifecycle_approve_with_exact_fee_split() {
    let (mut env, c, buyer, seller, _arbiter) = setup(&[500_000, 300_000]);
    env.send(&[&buyer], &[ix_fund(&c)]);
    assert_eq!(env.token_balance(&c.vault), 800_000);

    env.send(&[&seller], &[ix_submit(&c, &c.seller, 0, [1u8; 32])]);
    let (sb, tb) = (env.token_balance(&c.seller_ata), env.token_balance(&c.treasury_ata));
    let t0 = env.now();
    env.send(&[&buyer], &[ix_approve(&c, &c.buyer, 0)]);
    // fee = floor(500_000 * 250 / 10000) = 12_500
    assert_eq!(env.token_balance(&c.seller_ata) - sb, 487_500);
    assert_eq!(env.token_balance(&c.treasury_ata) - tb, 12_500);
    assert_eq!(env.token_balance(&c.vault), 300_000);

    let st = env.fetch_escrow(&c.escrow);
    assert_eq!(st.status, ES_FUNDED);
    assert_eq!(st.milestones[0].status, ST_RELEASED);
    assert_eq!(st.milestones[0].terminal_at, t0);
}

#[test]
fn timeout_claim_success_after_warp() {
    // The path localnet TS tests cannot reach: warp past the review window
    // and claim as seller, with exact balance assertions.
    let (mut env, c, buyer, seller, _arbiter) = setup(&[1_000_000]);
    env.send(&[&buyer], &[ix_fund(&c)]);
    env.send(&[&seller], &[ix_submit(&c, &c.seller, 0, [2u8; 32])]);
    let st = env.fetch_escrow(&c.escrow);
    let submitted_at = st.milestones[0].submitted_at;

    // One second before the deadline: claim fails, dispute succeeds.
    env.warp_to(submitted_at + REVIEW_WINDOW - 1);
    assert!(send_raw(&mut env.svm, &[&seller], &[ix_claim(&c, &c.seller, 0)]).is_err());
    env.send(&[&buyer], &[ix_dispute(&c, &c.buyer, 0)]);
    let st = env.fetch_escrow(&c.escrow);
    assert_eq!(st.milestones[0].status, ST_DISPUTED);

    // Fresh escrow for the success path (disputed one is consumed).
    let (mut env, c, buyer, seller, _arbiter) = setup(&[1_000_000]);
    env.send(&[&buyer], &[ix_fund(&c)]);
    env.send(&[&seller], &[ix_submit(&c, &c.seller, 0, [3u8; 32])]);
    let st = env.fetch_escrow(&c.escrow);
    // Exactly at the deadline: claim succeeds, dispute is too late.
    env.warp_to(st.milestones[0].submitted_at + REVIEW_WINDOW);
    assert!(send_raw(&mut env.svm, &[&buyer], &[ix_dispute(&c, &c.buyer, 0)]).is_err());
    let (sb, tb) = (env.token_balance(&c.seller_ata), env.token_balance(&c.treasury_ata));
    env.send(&[&seller], &[ix_claim(&c, &c.seller, 0)]);
    assert_eq!(env.token_balance(&c.seller_ata) - sb, 975_000);
    assert_eq!(env.token_balance(&c.treasury_ata) - tb, 25_000);
    assert_eq!(env.token_balance(&c.vault), 0);
    let st = env.fetch_escrow(&c.escrow);
    assert_eq!(st.status, ES_COMPLETED);
}

#[test]
fn treasury_rotation_only_affects_future_escrows() {
    // Fee AND treasury are snapshotted at creation (I5 extension).
    let (mut env, c, buyer, seller, _arbiter) = setup(&[400_000]);
    env.send(&[&buyer], &[ix_fund(&c)]);
    let new_treasury = env.actor().pubkey();
    let (admin, admin_pk) = (env.admin_clone(), env.admin_pubkey());
    env.send(&[&admin], &[ix_update_config(
        &env.config, &admin_pk, None, Some(new_treasury), None,
    )]);
    // Old escrow still pays the OLD treasury.
    env.send(&[&seller], &[ix_submit(&c, &c.seller, 0, [4u8; 32])]);
    let old_tb = env.token_balance(&c.treasury_ata);
    env.send(&[&buyer], &[ix_approve(&c, &c.buyer, 0)]);
    assert_eq!(env.token_balance(&c.treasury_ata) - old_tb, 10_000); // 2.5% of 400k
}

#[test]
fn mixed_8_milestone_lifecycle_ends_empty_and_completed() {
    // I11-flavored: approve some, timeout-claim one, dispute+resolve one,
    // reclaim the abandoned tail. Vault must end at exactly 0.
    let (mut env, c, buyer, seller, arbiter) =
        setup(&[100_000, 100_000, 100_000, 100_000, 100_000, 100_000, 100_000, 100_000]);
    env.send(&[&buyer], &[ix_fund(&c)]);
    assert_eq!(env.token_balance(&c.vault), 800_000);
    // m0: approve. m1: submit, warp, claim.
    env.send(&[&seller], &[ix_submit(&c, &c.seller, 0, [0u8; 32])]);
    env.send(&[&buyer], &[ix_approve(&c, &c.buyer, 0)]);
    env.send(&[&seller], &[ix_submit(&c, &c.seller, 1, [0u8; 32])]);
    let st = env.fetch_escrow(&c.escrow);
    env.warp_to(st.milestones[1].submitted_at + REVIEW_WINDOW);
    env.send(&[&seller], &[ix_claim(&c, &c.seller, 1)]);
    // m2: dispute then arbiter-resolve 50/50.
    env.send(&[&seller], &[ix_submit(&c, &c.seller, 2, [0u8; 32])]);
    env.send(&[&buyer], &[ix_dispute(&c, &c.buyer, 2)]);
    env.send(&[&arbiter], &[ix_resolve(&c, &c.arbiter, 2, 50_000, 50_000)]);
    // m3..m7 abandoned by seller: reclaim from m3 (cascades to m7).
    let st = env.fetch_escrow(&c.escrow);
    assert_eq!(st.milestones[2].status, ST_RESOLVED);
    env.warp_to(st.milestones[2].terminal_at + SELLER_DEADLINE);
    let bb = env.token_balance(&c.buyer_ata);
    env.send(&[&buyer], &[ix_reclaim(&c, &c.buyer, 3)]);
    assert_eq!(env.token_balance(&c.buyer_ata) - bb, 500_000);
    let st = env.fetch_escrow(&c.escrow);
    assert_eq!(st.status, ES_COMPLETED);
    assert_eq!(env.token_balance(&c.vault), 0);
    assert_eq!(st.released + st.refunded, st.total);
}
