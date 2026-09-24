//! Milestone type embedded in Escrow fixed array.

use anchor_lang::prelude::*;

#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace, Default, Debug,
)]
pub enum MilestoneStatus {
    #[default]
    Pending,
    Submitted,
    Released,
    Disputed,
    Resolved,
    Refunded,
    /// Closed without fund movement (e.g. cancel before funding). Never holds
    /// funds; terminal. Appended last so existing discriminants never shift.
    Cancelled,
}

/// Single milestone: amount fixed at creation, status transitions once to terminal.
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct Milestone {
    pub amount: u64,
    pub status: MilestoneStatus,
    pub submitted_at: i64,
    /// Set when the milestone reaches a terminal state; drives the
    /// seller-inactivity clock for the NEXT milestone (liveness, I11).
    pub terminal_at: i64,
    /// Set when a dispute is raised; drives the arbiter-timeout clock (I11).
    pub disputed_at: i64,
    pub evidence_hash: [u8; 32],
}

impl Milestone {
    /// Terminal = funds have left or been assigned (no further transitions).
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            MilestoneStatus::Released
                | MilestoneStatus::Resolved
                | MilestoneStatus::Refunded
                | MilestoneStatus::Cancelled
        )
    }

    /// Locked = still backed by vault balance.
    pub fn is_locked(&self) -> bool {
        matches!(
            self.status,
            MilestoneStatus::Pending | MilestoneStatus::Submitted | MilestoneStatus::Disputed
        )
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn m(status: MilestoneStatus) -> Milestone {
        Milestone {
            amount: 100,
            status,
            submitted_at: 0,
            terminal_at: 0,
            disputed_at: 0,
            evidence_hash: [0u8; 32],
        }
    }

    #[test]
    fn terminal_and_locked_partition_all_states() {
        // I3: every state is exactly one of terminal / locked (Submitted is locked, not terminal).
        for s in [
            MilestoneStatus::Pending,
            MilestoneStatus::Submitted,
            MilestoneStatus::Released,
            MilestoneStatus::Disputed,
            MilestoneStatus::Resolved,
            MilestoneStatus::Refunded,
            MilestoneStatus::Cancelled,
        ] {
            let ms = m(s);
            assert_ne!(
                ms.is_terminal(),
                ms.is_locked(),
                "{s:?} must be exactly one"
            );
        }
        assert!(!m(MilestoneStatus::Submitted).is_terminal());
    }
}
