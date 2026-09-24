//! Liveness exits on the real program (I11): seller abandonment =>
//! reclaim_stale_milestone; arbiter abandonment => expire_dispute 50/50.
//! Exact amounts, deadlines enforced both directions, odd-unit rounding.
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use solana_signer::Signer;

fn setup(amounts: &[u64]) -> (Env, Ctx, solana_keypair::Keypair, solana_keypair::Keypair, solana_keypair::Keypair) {
    let mut env = Env::new();
    let buyer = env.actor();
    let seller = env.actor();
    let arbiter = env.actor();
    let (escrow, _) = escrow_pda(&buyer.pubkey(), &seller.pubkey(), 21);
    let (vault, _) = vault_pda(&escrow);
    let buyer_ata = env.token_account(&buyer.pubkey(), amounts.iter().sum::<u64>() + 1_000_000);
    let seller_ata = env.token_account(&seller.pubkey(), 0);
    let t = env.treasury;
    let treasury_ata = env.token_account(&t, 0);
    let c = Ctx {
        buyer: buyer.pubkey(), seller: seller.pubkey(), arbiter: arbiter.pubkey(),
        mint: env.mint, escrow, vault, config: env.config,
        buyer_ata, seller_ata, treasury_ata,
    };
    env.send(&[&buyer], &[ix_create(
        &c, 21, amounts, REVIEW_WINDOW, SELLER_DEADLINE, ARBITER_TIMEOUT,
    )]);
    (env, c, buyer, seller, arbiter)
}

#[test]
fn reclaim_fails_before_deadline_succeeds_after() {
    let (mut env, c, buyer, seller, _arbiter) = setup(&[300_000, 300_000]);
    env.send(&[&buyer], &[ix_fund(&c)]);
    // m0 approved (progress), m1 never submitted by vanished seller.
    env.send(&[&seller], &[ix_submit(&c, &c.seller, 0, [0u8; 32])]);
    env.send(&[&buyer], &[ix_approve(&c, &c.buyer, 0)]);
    let st = env.fetch_escrow(&c.escrow);
    let active = st.milestones[1].status;
    assert_eq!(active, ST_PENDING);
    let since = st.milestones[0].terminal_at;
    // One second early: must fail.
    env.warp_to(since + SELLER_DEADLINE - 1);
    assert!(send_raw(&mut env.svm, &[&buyer], &[ix_reclaim(&c, &c.buyer, 1)]).is_err());
    // At the deadline: succeeds, exact refund, Completed.
    env.warp_to(since + SELLER_DEADLINE);
    let bb = env.token_balance(&c.buyer_ata);
    env.send(&[&buyer], &[ix_reclaim(&c, &c.buyer, 1)]);
    assert_eq!(env.token_balance(&c.buyer_ata) - bb, 300_000);
    let st = env.fetch_escrow(&c.escrow);
    assert_eq!(st.milestones[1].status, ST_REFUNDED);
    assert_eq!(st.status, ES_COMPLETED);
    assert_eq!(env.token_balance(&c.vault), 0);
}

#[test]
fn reclaim_requires_active_milestone() {
    // Cannot skip ahead: m1 not reclaimable while m0 is still Submitted.
    let (mut env, c, buyer, seller, _arbiter) = setup(&[300_000, 300_000]);
    env.send(&[&buyer], &[ix_fund(&c)]);
    env.send(&[&seller], &[ix_submit(&c, &c.seller, 0, [0u8; 32])]);
    env.warp_secs(SELLER_DEADLINE * 2);
    assert!(send_raw(&mut env.svm, &[&buyer], &[ix_reclaim(&c, &c.buyer, 1)]).is_err());
    // Non-buyer cannot reclaim: presenting the seller as `buyer` fails the
    // has_one check (I2, Anchor 2001).
    let seller_pk = seller.pubkey();
    let e = send_raw(&mut env.svm, &[&seller], &[ix_reclaim(&c, &seller_pk, 0)])
        .expect_err("non-buyer reclaim must fail");
    assert_eq!(err_code(&e), Some(2001));
}

#[test]
fn expire_dispute_5050_with_odd_unit_to_buyer() {
    let (mut env, c, buyer, seller, _arbiter) = setup(&[100_001]);
    env.send(&[&buyer], &[ix_fund(&c)]);
    env.send(&[&seller], &[ix_submit(&c, &c.seller, 0, [0u8; 32])]);
    env.send(&[&buyer], &[ix_dispute(&c, &c.buyer, 0)]);
    let st = env.fetch_escrow(&c.escrow);
    // Early: must fail.
    env.warp_to(st.milestones[0].disputed_at + ARBITER_TIMEOUT - 1);
    assert!(send_raw(&mut env.svm, &[&buyer], &[ix_expire(&c, &c.buyer, 0)]).is_err());
    // At timeout, SELLER triggers it (either party may).
    env.warp_to(st.milestones[0].disputed_at + ARBITER_TIMEOUT);
    let (sb, bb, tb) = (
        env.token_balance(&c.seller_ata),
        env.token_balance(&c.buyer_ata),
        env.token_balance(&c.treasury_ata),
    );
    env.send(&[&seller], &[ix_expire(&c, &c.seller, 0)]);
    // seller_leg = 50_000, fee = floor(50_000*250/10000) = 1_250.
    assert_eq!(env.token_balance(&c.seller_ata) - sb, 48_750);
    assert_eq!(env.token_balance(&c.treasury_ata) - tb, 1_250);
    assert_eq!(env.token_balance(&c.buyer_ata) - bb, 50_001); // odd unit to buyer
    let st = env.fetch_escrow(&c.escrow);
    assert_eq!(st.milestones[0].status, ST_RESOLVED);
    assert_eq!(st.status, ES_COMPLETED);
    assert_eq!(env.token_balance(&c.vault), 0);
}

#[test]
fn full_abandonment_chain_leaves_nothing_locked() {
    // Worst case: seller delivers m0 then vanishes; buyer disputes nothing;
    // arbiter irrelevant. Reclaim unwinds everything (I11 end-to-end).
    let (mut env, c, buyer, seller, _arbiter) = setup(&[250_000, 250_000, 250_000]);
    env.send(&[&buyer], &[ix_fund(&c)]);
    env.send(&[&seller], &[ix_submit(&c, &c.seller, 0, [0u8; 32])]);
    env.send(&[&buyer], &[ix_approve(&c, &c.buyer, 0)]);
    let st = env.fetch_escrow(&c.escrow);
    env.warp_to(st.milestones[0].terminal_at + SELLER_DEADLINE);
    let bb = env.token_balance(&c.buyer_ata);
    env.send(&[&buyer], &[ix_reclaim(&c, &c.buyer, 1)]);
    // m1 + m2 cascade-refunded.
    assert_eq!(env.token_balance(&c.buyer_ata) - bb, 500_000);
    let st = env.fetch_escrow(&c.escrow);
    assert_eq!(st.status, ES_COMPLETED);
    assert_eq!(env.token_balance(&c.vault), 0);
    assert_eq!(st.released + st.refunded, st.total);
}
