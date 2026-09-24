//! raise_dispute: buyer disputes Submitted milestone in-window.
use anchor_lang::prelude::*;

use crate::errors::EscrowError;
use crate::events::DisputeRaised;
use crate::state::{Escrow, EscrowStatus, MilestoneStatus};

#[derive(Accounts)]
pub struct RaiseDispute<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut, has_one = buyer)]
    pub escrow: Account<'info, Escrow>,
}

pub fn handler(ctx: Context<RaiseDispute>, index: u8) -> Result<()> {
    // I9: disputes are exits-adjacent — always allowed (no pause gate).
    require!(
        ctx.accounts.escrow.status == EscrowStatus::Funded,
        EscrowError::InvalidEscrowState
    );
    let idx = index as usize;
    require!(
        idx < ctx.accounts.escrow.milestone_count as usize,
        EscrowError::InvalidMilestoneIndex
    );
    require!(
        ctx.accounts.escrow.milestones[idx].status == MilestoneStatus::Submitted,
        EscrowError::InvalidMilestoneState
    );

    // Must be within review window: now < submitted_at + window.
    // Boundary: disputing at exactly deadline is too late (seller can claim).
    let now = Clock::get()?.unix_timestamp;
    let submitted_at = ctx.accounts.escrow.milestones[idx].submitted_at;
    let window = ctx.accounts.escrow.review_window_secs;
    let deadline = submitted_at
        .checked_add(window)
        .ok_or(error!(EscrowError::MathOverflow))?;
    require!(now < deadline, EscrowError::WindowElapsed);

    let escrow_key = ctx.accounts.escrow.key();
    let buyer_key = ctx.accounts.buyer.key();
    ctx.accounts.escrow.milestones[idx].status = MilestoneStatus::Disputed;
    // Starts the arbiter-timeout clock (liveness exit, I11).
    ctx.accounts.escrow.milestones[idx].disputed_at = now;

    emit!(DisputeRaised {
        escrow: escrow_key,
        index,
        raised_by: buyer_key,
    });
    Ok(())
}
