//! Events emitted on every state change for off-chain indexing.

use anchor_lang::prelude::*;

#[event]
pub struct ConfigInitialized {
    pub admin: Pubkey,
    pub treasury: Pubkey,
    pub fee_bps: u16,
}

#[event]
pub struct PausedUpdated {
    pub paused: bool,
}

#[event]
pub struct ConfigUpdated {
    pub fee_bps: u16,
    pub treasury: Pubkey,
    pub allowlist_len: u8,
}

#[event]
pub struct EscrowCreated {
    pub escrow: Pubkey,
    pub buyer: Pubkey,
    pub seller: Pubkey,
    pub arbiter: Pubkey,
    pub total_amount: u64,
    pub milestone_count: u8,
}

#[event]
pub struct EscrowFunded {
    pub escrow: Pubkey,
    pub amount: u64,
}

#[event]
pub struct MilestoneSubmitted {
    pub escrow: Pubkey,
    pub index: u8,
    pub evidence_hash: [u8; 32],
}

#[event]
pub struct MilestoneReleased {
    pub escrow: Pubkey,
    pub index: u8,
    pub seller_amount: u64,
    pub fee_amount: u64,
    pub reason: ReleaseReason,
}

#[event]
pub struct DisputeRaised {
    pub escrow: Pubkey,
    pub index: u8,
    pub raised_by: Pubkey,
}

#[event]
pub struct DisputeResolved {
    pub escrow: Pubkey,
    pub index: u8,
    pub seller_amount: u64,
    pub buyer_amount: u64,
}

#[event]
pub struct EscrowCancelled {
    pub escrow: Pubkey,
}

#[event]
pub struct EscrowClosed {
    pub escrow: Pubkey,
}

#[event]
pub struct MilestoneReclaimed {
    pub escrow: Pubkey,
    pub from_index: u8,
    pub milestone_count: u8,
    pub refunded_amount: u64,
}

#[event]
pub struct DisputeExpired {
    pub escrow: Pubkey,
    pub index: u8,
    pub seller_amount: u64,
    pub buyer_amount: u64,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseReason {
    Approved,
    Timeout,
}
