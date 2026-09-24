//! Input + role validation.
use anchor_lang::prelude::*;

use crate::constants::{
    MAX_ALLOWED_MINTS, MAX_ARBITER_TIMEOUT_SECS, MAX_FEE_BPS, MAX_MILESTONES,
    MAX_REVIEW_WINDOW_SECS, MAX_SELLER_DEADLINE_SECS, MIN_ARBITER_TIMEOUT_SECS,
    MIN_REVIEW_WINDOW_SECS, MIN_SELLER_DEADLINE_SECS,
};
use crate::errors::EscrowError;

/// Buyer, seller, arbiter must be pairwise distinct (I6).
pub fn assert_distinct_roles(buyer: &Pubkey, seller: &Pubkey, arbiter: &Pubkey) -> Result<()> {
    require!(
        buyer != seller && buyer != arbiter && seller != arbiter,
        EscrowError::DuplicateRole
    );
    Ok(())
}

/// Validate milestone vector: 1..=8, each > 0.
pub fn assert_valid_milestones(amounts: &[u64]) -> Result<()> {
    require!(
        !amounts.is_empty() && amounts.len() <= MAX_MILESTONES,
        EscrowError::InvalidMilestoneCount
    );
    for a in amounts {
        require!(*a > 0, EscrowError::ZeroMilestoneAmount);
    }
    Ok(())
}

/// Validate fee bps ceiling.
pub fn assert_valid_fee(fee_bps: u16) -> Result<()> {
    require!(fee_bps <= MAX_FEE_BPS, EscrowError::FeeTooHigh);
    Ok(())
}

/// Validate review window bounds.
pub fn assert_valid_window(window_secs: i64) -> Result<()> {
    require!(
        (MIN_REVIEW_WINDOW_SECS..=MAX_REVIEW_WINDOW_SECS).contains(&window_secs),
        EscrowError::InvalidReviewWindow
    );
    Ok(())
}

/// Validate allowlist size.
pub fn assert_valid_allowlist(mints: &[Pubkey]) -> Result<()> {
    require!(
        !mints.is_empty() && mints.len() <= MAX_ALLOWED_MINTS,
        EscrowError::MintNotAllowed
    );
    Ok(())
}

/// Validate seller-inactivity deadline bounds (liveness exit, I11).
pub fn assert_valid_seller_deadline(deadline_secs: i64) -> Result<()> {
    require!(
        (MIN_SELLER_DEADLINE_SECS..=MAX_SELLER_DEADLINE_SECS).contains(&deadline_secs),
        EscrowError::InvalidReviewWindow
    );
    Ok(())
}

/// Validate arbiter-timeout bounds (liveness exit, I11).
pub fn assert_valid_arbiter_timeout(timeout_secs: i64) -> Result<()> {
    require!(
        (MIN_ARBITER_TIMEOUT_SECS..=MAX_ARBITER_TIMEOUT_SECS).contains(&timeout_secs),
        EscrowError::InvalidReviewWindow
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;

    fn key(n: u8) -> Pubkey {
        Pubkey::new_from_array([n; 32])
    }

    #[test]
    fn distinct_roles_ok() {
        assert!(assert_distinct_roles(&key(1), &key(2), &key(3)).is_ok());
    }

    #[test]
    fn duplicate_roles_rejected() {
        // I6: every pairwise collision must fail.
        assert!(assert_distinct_roles(&key(1), &key(1), &key(3)).is_err());
        assert!(assert_distinct_roles(&key(1), &key(2), &key(1)).is_err());
        assert!(assert_distinct_roles(&key(1), &key(2), &key(2)).is_err());
    }

    #[test]
    fn milestone_counts() {
        assert!(assert_valid_milestones(&[1]).is_ok());
        assert!(assert_valid_milestones(&[1, 2, 3, 4, 5, 6, 7, 8]).is_ok());
        assert!(assert_valid_milestones(&[]).is_err());
        assert!(assert_valid_milestones(&[1, 2, 3, 4, 5, 6, 7, 8, 9]).is_err());
        assert!(assert_valid_milestones(&[100, 0]).is_err());
    }

    #[test]
    fn fee_bounds() {
        assert!(assert_valid_fee(0).is_ok());
        assert!(assert_valid_fee(500).is_ok());
        assert!(assert_valid_fee(501).is_err());
    }

    #[test]
    fn window_bounds() {
        assert!(assert_valid_window(MIN_REVIEW_WINDOW_SECS).is_ok());
        assert!(assert_valid_window(MAX_REVIEW_WINDOW_SECS).is_ok());
        assert!(assert_valid_window(MIN_REVIEW_WINDOW_SECS - 1).is_err());
        assert!(assert_valid_window(MAX_REVIEW_WINDOW_SECS + 1).is_err());
        assert!(assert_valid_window(0).is_err());
    }

    #[test]
    fn allowlist_bounds() {
        assert!(assert_valid_allowlist(&[]).is_err());
        assert!(assert_valid_allowlist(&[key(1)]).is_ok());
        assert!(assert_valid_allowlist(&[key(1), key(2), key(3), key(4)]).is_ok());
        assert!(assert_valid_allowlist(&[key(1), key(2), key(3), key(4), key(5)]).is_err());
    }

    #[test]
    fn liveness_window_bounds() {
        assert!(assert_valid_seller_deadline(MIN_SELLER_DEADLINE_SECS).is_ok());
        assert!(assert_valid_seller_deadline(MAX_SELLER_DEADLINE_SECS).is_ok());
        assert!(assert_valid_seller_deadline(MIN_SELLER_DEADLINE_SECS - 1).is_err());
        assert!(assert_valid_seller_deadline(MAX_SELLER_DEADLINE_SECS + 1).is_err());
        assert!(assert_valid_arbiter_timeout(MIN_ARBITER_TIMEOUT_SECS).is_ok());
        assert!(assert_valid_arbiter_timeout(MAX_ARBITER_TIMEOUT_SECS).is_ok());
        assert!(assert_valid_arbiter_timeout(MIN_ARBITER_TIMEOUT_SECS - 1).is_err());
        assert!(assert_valid_arbiter_timeout(MAX_ARBITER_TIMEOUT_SECS + 1).is_err());
    }
}
