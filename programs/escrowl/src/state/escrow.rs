//! Escrow PDA: `["escrow", buyer, seller, escrow_id_le]`.

use anchor_lang::prelude::*;

use super::milestone::{Milestone, MilestoneStatus};
use crate::errors::EscrowError;

#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Default, Debug,
)]
pub enum EscrowStatus {
    #[default]
    Created,
    Funded,
    Completed,
    Cancelled,
}

/// Custodied escrow terms. Vault authority = this PDA.
#[account]
#[derive(InitSpace)]
pub struct Escrow {
    pub buyer: Pubkey,
    pub seller: Pubkey,
    pub arbiter: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub escrow_id: u64,
    pub milestone_count: u8,
    #[max_len(8)]
    pub milestones: Vec<Milestone>,
    pub total_amount: u64,
    pub released_amount: u64,
    pub refunded_amount: u64,
    /// Fee snapshot at creation; later Config changes don't affect open escrows.
    pub fee_bps_snapshot: u16,
    /// Treasury snapshot at creation; rotations only affect future escrows.
    pub treasury_snapshot: Pubkey,
    pub review_window_secs: i64,
    /// Buyer reclaims a stale Pending milestone after this long without
    /// seller progress (liveness, I11).
    pub seller_deadline_secs: i64,
    /// Either party unlocks a Disputed milestone after this long without
    /// arbiter resolution (liveness, I11).
    pub arbiter_timeout_secs: i64,
    pub status: EscrowStatus,
    pub created_at: i64,
    /// Set once at funding; milestone 0's inactivity clock starts here.
    pub funded_at: i64,
    pub bump: u8,
    pub vault_bump: u8,
}

impl Escrow {
    /// Sum of locked milestone amounts; must equal vault balance while
    /// Funded (I1). Checked math: overflow is an error, never silent.
    pub fn locked_amount(&self) -> Result<u64> {
        let mut total: u64 = 0;
        for m in self.milestones.iter().filter(|m| m.is_locked()) {
            total = total
                .checked_add(m.amount)
                .ok_or(error!(EscrowError::MathOverflow))?;
        }
        Ok(total)
    }

    pub fn all_terminal(&self) -> bool {
        self.milestones.iter().all(|m| m.is_terminal())
    }

    pub fn any_submitted_or_beyond(&self) -> bool {
        self.milestones
            .iter()
            .any(|m| !matches!(m.status, MilestoneStatus::Pending))
    }

    /// Timestamp from which milestone `idx`'s seller-inactivity clock runs:
    /// funding time for index 0, previous milestone's terminal time after.
    /// Returns None if a prior milestone is not terminal (not yet active).
    pub fn milestone_active_since(&self, idx: usize) -> Option<i64> {
        if idx >= self.milestones.len() {
            return None;
        }
        if idx == 0 {
            return Some(self.funded_at);
        }
        let prev = &self.milestones[idx - 1];
        if prev.is_terminal() {
            Some(prev.terminal_at)
        } else {
            None
        }
    }
}
