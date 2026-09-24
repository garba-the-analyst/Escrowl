//! Custom program errors. Every failure maps here — no silent panics.
//!
//! Codes are sequential from 6000 in declaration order. Variants are never
//! removed or reordered once deployed (that would shift every code after
//! them); `NotImplemented` was deleted pre-mainnet and all consumers updated.

use anchor_lang::prelude::*;

#[error_code]
pub enum EscrowError {
    #[msg("Invalid milestone count: must be 1..=8")]
    InvalidMilestoneCount,
    #[msg("Milestone amount must be > 0")]
    ZeroMilestoneAmount,
    #[msg("Sum of milestones must equal total amount")]
    TotalMismatch,
    #[msg("Arithmetic overflow")]
    MathOverflow,
    #[msg("Buyer, seller and arbiter must be pairwise distinct")]
    DuplicateRole,
    #[msg("Mint not in allowlist")]
    MintNotAllowed,
    #[msg("Token-2022 mints are rejected; SPL classic only")]
    Token2022Rejected,
    #[msg("Invalid milestone index")]
    InvalidMilestoneIndex,
    #[msg("Milestones must be submitted in order")]
    OutOfOrderSubmit,
    #[msg("Milestone not in expected state for this action")]
    InvalidMilestoneState,
    #[msg("Escrow not in expected state for this action")]
    InvalidEscrowState,
    #[msg("Review window has not elapsed yet")]
    WindowNotElapsed,
    #[msg("Review window already elapsed")]
    WindowElapsed,
    #[msg("Dispute split must sum exactly to milestone amount")]
    InvalidSplitSum,
    #[msg("Fee exceeds maximum (500 bps)")]
    FeeTooHigh,
    #[msg("Protocol is paused for create/fund")]
    Paused,
    #[msg("Cannot cancel after work has started")]
    CancelNotAllowed,
    #[msg("Escrow has no terminal state on all milestones yet")]
    NotAllTerminal,
    #[msg("Vault balance does not match expected amount")]
    VaultBalanceMismatch,
    #[msg("Vault must be empty for this action")]
    VaultNotEmpty,
    #[msg("Unauthorized: wrong role for this action")]
    Unauthorized,
    #[msg("Invalid review window")]
    InvalidReviewWindow,
    #[msg("Mint already allowlisted or allowlist full")]
    AllowlistUpdateInvalid,
    #[msg("Seller inactivity deadline has not elapsed yet")]
    SellerDeadlineNotElapsed,
    #[msg("Arbiter timeout has not elapsed yet")]
    DisputeNotExpired,
}
