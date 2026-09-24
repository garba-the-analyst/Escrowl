#![forbid(unsafe_code)]
// Anchor's `#[program]` macro resolves account types through these glob
// imports; listing them explicitly breaks the build (see DECISIONS.md).
#![allow(clippy::wildcard_imports)]
//! Escrowl: milestone-based, dispute-aware SPL escrow on Solana.
//!
//! Twelve instructions with full account validation; security posture:
//! no unwrap/expect, checked math only, SPL classic only,
//! distinct buyer/seller/arbiter enforced at creation.
//! Invariants live in docs/INVARIANTS.md; threat model in docs/THREAT_MODEL.md.

use anchor_lang::prelude::*;

pub mod constants;
pub mod errors;
pub mod events;
pub mod instructions;
pub mod state;
pub mod utils;

use instructions::*;

declare_id!("61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE");

#[program]
pub mod escrowl {
    use super::*;

    /// Create global config (admin, treasury, fee, mint allowlist).
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        fee_bps: u16,
        allowed_mints: Vec<Pubkey>,
    ) -> Result<()> {
        instructions::initialize_config::handler(ctx, fee_bps, allowed_mints)
    }

    /// Pause/unpause create + fund only. Exits are never blocked.
    pub fn set_paused(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
        instructions::set_paused::handler(ctx, paused)
    }

    /// Admin maintenance: rotate fee/treasury, append to mint allowlist.
    /// Open escrows unaffected (fee snapshotted at creation).
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        new_fee_bps: Option<u16>,
        new_treasury: Option<Pubkey>,
        add_mint: Option<Pubkey>,
    ) -> Result<()> {
        instructions::update_config::handler(ctx, new_fee_bps, new_treasury, add_mint)
    }

    /// Buyer creates escrow terms with 1..=8 milestone amounts.
    pub fn create_escrow(
        ctx: Context<CreateEscrow>,
        escrow_id: u64,
        milestone_amounts: Vec<u64>,
        review_window_secs: i64,
        seller_deadline_secs: i64,
        arbiter_timeout_secs: i64,
    ) -> Result<()> {
        instructions::create_escrow::handler(
            ctx,
            escrow_id,
            milestone_amounts,
            review_window_secs,
            seller_deadline_secs,
            arbiter_timeout_secs,
        )
    }

    /// Buyer funds vault with exact total_amount. Created -> Funded, once.
    pub fn fund_escrow(ctx: Context<FundEscrow>) -> Result<()> {
        instructions::fund_escrow::handler(ctx)
    }

    /// Seller submits milestone in order with evidence hash.
    pub fn submit_milestone(
        ctx: Context<SubmitMilestone>,
        index: u8,
        evidence_hash: [u8; 32],
    ) -> Result<()> {
        instructions::submit_milestone::handler(ctx, index, evidence_hash)
    }

    /// Buyer approves submitted milestone, splits seller/treasury by fee.
    pub fn approve_milestone(ctx: Context<ApproveMilestone>, index: u8) -> Result<()> {
        instructions::approve_milestone::handler(ctx, index)
    }

    /// Seller claims after buyer review window elapsed without dispute.
    pub fn claim_after_timeout(ctx: Context<ClaimAfterTimeout>, index: u8) -> Result<()> {
        instructions::claim_after_timeout::handler(ctx, index)
    }

    /// Buyer disputes a Submitted milestone within review window.
    pub fn raise_dispute(ctx: Context<RaiseDispute>, index: u8) -> Result<()> {
        instructions::raise_dispute::handler(ctx, index)
    }

    /// Arbiter resolves with exact split seller_amount + buyer_amount == milestone.
    pub fn resolve_dispute(
        ctx: Context<ResolveDispute>,
        index: u8,
        seller_amount: u64,
        buyer_amount: u64,
    ) -> Result<()> {
        instructions::resolve_dispute::handler(ctx, index, seller_amount, buyer_amount)
    }

    /// Buyer cancels if Created or Funded with zero milestones ever submitted.
    pub fn cancel_escrow(ctx: Context<CancelEscrow>) -> Result<()> {
        instructions::cancel_escrow::handler(ctx)
    }

    /// Buyer refunds a stale Pending milestone (seller vanished) plus later
    /// Pending ones, after the seller deadline. Liveness exit (I11).
    pub fn reclaim_stale_milestone(ctx: Context<ReclaimStaleMilestone>, index: u8) -> Result<()> {
        instructions::reclaim_stale_milestone::handler(ctx, index)
    }

    /// Either counterparty unlocks an abandoned dispute 50/50 after the
    /// arbiter timeout. Liveness exit (I11).
    pub fn expire_dispute(ctx: Context<ExpireDispute>, index: u8) -> Result<()> {
        instructions::expire_dispute::handler(ctx, index)
    }

    /// Close vault + escrow when all milestones terminal, rent to buyer.
    pub fn close_escrow(ctx: Context<CloseEscrow>) -> Result<()> {
        instructions::close_escrow::handler(ctx)
    }
}
