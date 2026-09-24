//! Model-based state-machine checks: random op sequences against a faithful
//! mirror of the on-chain transition rules, asserting conservation after
//! EVERY op. HONEST SCOPE: this validates the fee math, validation helpers,
//! and state predicates (real crate code) plus a mirrored transition model —
//! it is NOT program fuzzing and never executes an instruction handler. Real
//! on-chain randomized coverage lives in tests-litesvm/ (LiteSVM harness
//! driving the compiled program). Run: `cargo test -p escrowl`.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use escrowl::state::{Milestone, MilestoneStatus};
use escrowl::utils::{calc_fee, calc_seller_amount};

/// Deterministic xorshift64 — no external deps.
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
        if n == 0 {
            0
        } else {
            self.next() % n
        }
    }
}

struct Model {
    milestones: Vec<Milestone>,
    total: u64,
    vault: u64,
    buyer_out: u64, // buyer balance outside vault (starts at total)
    seller_out: u64,
    treasury_out: u64,
    released: u64,
    refunded: u64,
    now: i64,
    window: i64,
    fee_bps: u16,
    funded: bool,
}

impl Model {
    fn new(rng: &mut Rng) -> Self {
        let n = 1 + rng.below(8) as usize;
        let mut milestones = Vec::new();
        let mut total = 0u64;
        for _ in 0..n {
            // Amounts span dust (1) to large (1e12) to stress fee flooring.
            let a = match rng.below(4) {
                0 => 1 + rng.below(10),
                1 => 1 + rng.below(10_000),
                2 => 1 + rng.below(1_000_000),
                _ => 1 + rng.below(1_000_000_000_000),
            };
            total = total.checked_add(a).unwrap();
            milestones.push(Milestone {
                amount: a,
                status: MilestoneStatus::Pending,
                submitted_at: 0,
                terminal_at: 0,
                disputed_at: 0,
                evidence_hash: [0u8; 32],
            });
        }
        let fee_bps = [0u16, 1, 100, 250, 500][rng.below(5) as usize];
        Self {
            milestones,
            total,
            vault: 0,
            buyer_out: total,
            seller_out: 0,
            treasury_out: 0,
            released: 0,
            refunded: 0,
            now: 1_700_000_000,
            window: 3_600 + rng.below(100_000) as i64,
            fee_bps,
            funded: false,
        }
    }

    /// I1 + conservation, checked after every op. I1 is scoped: it holds
    /// once funded (in Created the vault is legitimately empty pre-deposit).
    fn check_invariants(&self) {
        let locked: u64 = self
            .milestones
            .iter()
            .filter(|m| m.is_locked())
            .map(|m| m.amount)
            .sum();
        if self.funded {
            assert_eq!(self.vault, locked, "I1: vault == locked (funded)");
        } else {
            assert_eq!(self.vault, 0, "unfunded vault must be empty");
        }
        assert_eq!(
            self.vault + self.buyer_out + self.seller_out + self.treasury_out,
            self.total,
            "conservation: all funds accounted"
        );
        assert_eq!(
            self.released + self.refunded + locked,
            self.total,
            "released + refunded + locked == total"
        );
        // Terminal milestones never hold funds.
        for m in &self.milestones {
            if m.is_terminal() {
                assert!(!m.is_locked());
            }
        }
    }

    fn fund(&mut self) -> bool {
        if self.funded || self.vault != 0 {
            return false;
        }
        self.buyer_out -= self.total;
        self.vault += self.total;
        self.funded = true;
        true
    }

    fn submit(&mut self, i: usize) -> bool {
        if !self.funded || i >= self.milestones.len() {
            return false;
        }
        if self.milestones[i].status != MilestoneStatus::Pending {
            return false;
        }
        if self.milestones[..i]
            .iter()
            .any(|m| m.status == MilestoneStatus::Pending)
        {
            return false; // in-order rule
        }
        self.milestones[i].status = MilestoneStatus::Submitted;
        self.milestones[i].submitted_at = self.now;
        true
    }

    /// Shared payout path for approve / timeout-claim.
    fn payout(&mut self, i: usize) -> bool {
        let m = self.milestones[i];
        let fee = calc_fee(m.amount, self.fee_bps).unwrap();
        let proceeds = calc_seller_amount(m.amount, self.fee_bps).unwrap();
        assert_eq!(fee + proceeds, m.amount, "I5: fee + seller == leg");
        self.vault -= m.amount;
        self.seller_out += proceeds;
        self.treasury_out += fee;
        self.milestones[i].status = MilestoneStatus::Released;
        self.released += m.amount;
        true
    }

    fn approve(&mut self, i: usize) -> bool {
        if !self.funded || i >= self.milestones.len() {
            return false;
        }
        if self.milestones[i].status != MilestoneStatus::Submitted {
            return false;
        }
        self.payout(i)
    }

    fn claim(&mut self, i: usize) -> bool {
        if !self.funded || i >= self.milestones.len() {
            return false;
        }
        let m = self.milestones[i];
        if m.status != MilestoneStatus::Submitted {
            return false;
        }
        if self.now < m.submitted_at + self.window {
            return false;
        }
        self.payout(i)
    }

    fn dispute(&mut self, i: usize) -> bool {
        if !self.funded || i >= self.milestones.len() {
            return false;
        }
        let m = self.milestones[i];
        if m.status != MilestoneStatus::Submitted {
            return false;
        }
        if self.now >= m.submitted_at + self.window {
            return false;
        }
        self.milestones[i].status = MilestoneStatus::Disputed;
        true
    }

    /// Returns false (rejected) unless split sums exactly — mirrors I4.
    fn resolve(&mut self, i: usize, seller_leg: u64, buyer_leg: u64) -> bool {
        if !self.funded || i >= self.milestones.len() {
            return false;
        }
        let m = self.milestones[i];
        if m.status != MilestoneStatus::Disputed {
            return false;
        }
        if seller_leg.checked_add(buyer_leg) != Some(m.amount) {
            return false; // I4: exact split required
        }
        let fee = calc_fee(seller_leg, self.fee_bps).unwrap();
        let proceeds = calc_seller_amount(seller_leg, self.fee_bps).unwrap();
        self.vault -= m.amount;
        self.seller_out += proceeds;
        self.treasury_out += fee;
        self.buyer_out += buyer_leg;
        self.milestones[i].status = MilestoneStatus::Resolved;
        self.released += seller_leg;
        self.refunded += buyer_leg;
        true
    }
}

fn run_seed(seed: u64) {
    let mut rng = Rng(seed.max(1));
    let mut model = Model::new(&mut rng);
    model.check_invariants();

    // Sometimes skip funding to exercise unfunded rejections.
    if rng.below(10) != 0 {
        assert!(model.fund());
    }
    model.check_invariants();

    for _ in 0..400 {
        let n = model.milestones.len() as u64;
        let i = rng.below(n) as usize;
        match rng.below(7) {
            0 => {
                model.submit(i);
            }
            1 => {
                model.approve(i);
            }
            2 => {
                model.claim(i);
            }
            3 => {
                model.dispute(i);
            }
            4 => {
                // Resolve with random split; frequently wrong on purpose (I4).
                let amt = model.milestones[i].amount;
                let s = if rng.below(3) == 0 {
                    rng.below(amt + 2) // may overshoot -> must reject
                } else {
                    rng.below(amt + 1) // valid leg, buyer gets rest
                };
                let b = rng.below(amt + 2);
                let (s, b) = if rng.below(2) == 0 && s <= amt {
                    (s, amt - s) // exact split
                } else {
                    (s, b) // possibly inexact
                };
                let applied = model.resolve(i, s, b);
                // I4 oracle: applied iff disputed-at-entry is unobservable here,
                // so only assert the arithmetic direction when applied.
                if applied {
                    assert_eq!(s + b, amt);
                }
            }
            5 => {
                // Clock warp forward (covers timeout + late-dispute edges).
                model.now += rng.below(model.window as u64 * 2 + 1) as i64;
            }
            _ => {
                // Deliberate double-transition attempts (I3): hammer the
                // same milestone with every mutating op; invariants must hold.
                model.submit(i);
                model.approve(i);
                model.claim(i);
                model.dispute(i);
            }
        }
        model.check_invariants();
    }

    // End-of-run accounting: nothing created or destroyed.
    assert_eq!(
        model.vault + model.buyer_out + model.seller_out + model.treasury_out,
        model.total
    );
}

#[test]
fn fuzz_state_machine_seeds() {
    for seed in 1..=200u64 {
        run_seed(seed);
    }
}

#[test]
fn fuzz_timeout_claim_path_reached() {
    // Targeted: warp far past the window and prove claim() succeeds and
    // conserves funds — the path localnet TS tests cannot reach.
    let mut reached = 0;
    for seed in 1..=50u64 {
        let mut rng = Rng(1_000_000 + seed);
        let mut model = Model::new(&mut rng);
        model.fund();
        model.submit(0);
        assert!(!model.claim(0), "claim before deadline must fail");
        model.now += model.window + 1;
        assert!(model.claim(0), "claim after deadline must succeed");
        model.check_invariants();
        reached += 1;
    }
    assert_eq!(reached, 50);
}

#[test]
fn fuzz_late_dispute_rejected() {
    // Disputing at/after the deadline must always fail.
    for seed in 1..=50u64 {
        let mut rng = Rng(2_000_000 + seed);
        let mut model = Model::new(&mut rng);
        model.fund();
        model.submit(0);
        model.now += model.window; // exactly at deadline
        assert!(!model.dispute(0), "dispute at deadline must fail");
        model.now += 1;
        assert!(!model.dispute(0), "dispute past deadline must fail");
        model.check_invariants();
    }
}
