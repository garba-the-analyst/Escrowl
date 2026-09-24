//! Randomized differential fuzz over the REAL program: seeded sequences of
//! valid AND invalid instructions (200 seeds x 100 ops). After every op:
//! conservation (I12: vault+seller+buyer+treasury == funded) and vault ==
//! locked while Funded (I1). Rejected instructions must leave ALL state and
//! balances bit-identical (state-hash before/after).
#![allow(clippy::unwrap_used)]

mod common;

use common::*;
use solana_keypair::Keypair;
use solana_signer::Signer;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x.max(1);
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        if n == 0 { 0 } else { self.next() % n }
    }
}

fn snapshot(env: &Env, c: &Ctx) -> Vec<u8> {
    let mut s = env.svm.get_account(&c.escrow).map(|a| a.data).unwrap_or_default();
    for ata in [&c.vault, &c.seller_ata, &c.buyer_ata, &c.treasury_ata] {
        match env.svm.get_account(ata) {
            Some(a) => s.extend_from_slice(&a.data),
            None => s.extend_from_slice(&[0xFF]),
        }
    }
    s
}

fn run_seed(seed: u64) {
    let mut rng = Rng(seed.max(1));
    let mut env = Env::new();
    let buyer = env.actor();
    let seller = env.actor();
    let arbiter = env.actor();
    let attacker = env.actor();
    let n = 1 + rng.below(4) as usize;
    let mut amounts = Vec::new();
    for _ in 0..n {
        amounts.push(1 + rng.below(1_000_000));
    }
    let total: u64 = amounts.iter().sum();
    let id = 1000 + seed % 50000;
    let (escrow, _) = escrow_pda(&buyer.pubkey(), &seller.pubkey(), id);
    let (vault, _) = vault_pda(&escrow);
    let buyer_ata = env.token_account(&buyer.pubkey(), total + 50_000);
    let seller_ata = env.token_account(&seller.pubkey(), 0);
    let t = env.treasury;
    let treasury_ata = env.token_account(&t, 0);
    let c = Ctx {
        buyer: buyer.pubkey(), seller: seller.pubkey(), arbiter: arbiter.pubkey(),
        mint: env.mint, escrow, vault, config: env.config,
        buyer_ata, seller_ata, treasury_ata,
    };
    let funded_total = total;

    for _ in 0..100 {
        let i = rng.below(8) as u8;
        let op = rng.below(11);
        let do_it: Option<(Vec<&Keypair>, solana_instruction::Instruction)> = match op {
            0 => Some((vec![&buyer], ix_create(&c, id, &amounts, REVIEW_WINDOW, SELLER_DEADLINE, ARBITER_TIMEOUT))),
            1 => Some((vec![&buyer], ix_fund(&c))),
            2 => Some((vec![&seller], ix_submit(&c, &c.seller, i, [7u8; 32]))),
            3 => Some((vec![&buyer], ix_approve(&c, &c.buyer, i))),
            4 => Some((vec![&seller], ix_claim(&c, &c.seller, i))),
            5 => Some((vec![&buyer], ix_dispute(&c, &c.buyer, i))),
            6 => {
                let amt = amounts.get(i as usize).copied().unwrap_or(1000);
                let s = rng.below(amt + 3); // sometimes overshoots
                let b = rng.below(amt + 3);
                let (s, b) = if rng.below(2) == 0 && s <= amt { (s, amt - s) } else { (s, b) };
                Some((vec![&arbiter], ix_resolve(&c, &c.arbiter, i, s, b)))
            }
            7 => Some((vec![&buyer], ix_cancel(&c, &c.buyer))),
            8 => Some((vec![&buyer], ix_reclaim(&c, &c.buyer, i))),
            9 => {
                let who = if rng.below(2) == 0 { &buyer } else { &seller };
                Some((vec![who], ix_expire(&c, &who.pubkey(), i)))
            }
            _ => {
                // Attacker-flavored: wrong signer / close attempt.
                if rng.below(2) == 0 {
                    Some((vec![&attacker], ix_approve(&c, &attacker.pubkey(), i)))
                } else {
                    Some((vec![&buyer], ix_close(&c, &c.buyer)))
                }
            }
        };
        // Random clock warp (drives timeouts, deadlines, expiries).
        if rng.below(4) == 0 {
            env.warp_secs(rng.below(200_000) as i64);
        }
        let (signers, ix) = do_it.unwrap();
        let before = snapshot(&env, &c);
        let res = send_raw(&mut env.svm, &signers, &[ix]);
        if res.is_err() {
            // Rejected: NOTHING may change (atomicity).
            assert_eq!(snapshot(&env, &c), before, "rejected ix mutated state (seed {seed})");
        } else {
            // Accepted: exact conservation across the four pots (I12).
            // Buyer started with total+50_000; seller/treasury/vault at 0.
            // (A closed vault reads 0; disappearance of funds still breaks
            // the sum below.)
            let pots = Pots::read(&env, &c);
            assert_eq!(pots.sum(), funded_total + 50_000, "I12 conservation (seed {seed})");
            if let Some(st) = env.fetch_escrow_opt(&c.escrow) {
                assert_eq!(st.released + st.refunded + st.locked() + st.cancelled(), st.total, "I12 ledger (seed {seed})");
                if st.status == ES_FUNDED {
                    assert_eq!(pots.vault, st.locked(), "I1 vault==locked (seed {seed})");
                }
            }
        }
    }
    // End of run: global conservation regardless of path taken.
    let pots = Pots::read(&env, &c);
    assert_eq!(pots.sum(), funded_total + 50_000, "final conservation (seed {seed})");
    if let Some(st) = env.fetch_escrow_opt(&c.escrow) {
        assert_eq!(st.released + st.refunded + st.locked() + st.cancelled(), st.total);
    }
}

fn run_range(from: u64, to: u64) {
    for seed in from..=to {
        run_seed(seed);
    }
}

// 200 seeds x 100 ops split across 4 tests so cargo runs them on
// parallel threads (each seed builds a fresh SVM).
#[test]
fn fuzz_random_sequences_001_050() {
    run_range(1, 50);
}

#[test]
fn fuzz_random_sequences_051_100() {
    run_range(51, 100);
}

#[test]
fn fuzz_random_sequences_101_150() {
    run_range(101, 150);
}

#[test]
fn fuzz_random_sequences_151_200() {
    run_range(151, 200);
}
