//! submit_milestone: seller submits in order with evidence hash.
use anchor_lang::prelude::*;

use crate::errors::EscrowError;
use crate::events::MilestoneSubmitted;
use crate::state::{Escrow, EscrowStatus, MilestoneStatus};

#[derive(Accounts)]
pub struct SubmitMilestone<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    #[account(mut, has_one = seller)]
    pub escrow: Account<'info, Escrow>,
}

pub fn handler(ctx: Context<SubmitMilestone>, index: u8, evidence_hash: [u8; 32]) -> Result<()> {
    require!(
        ctx.accounts.escrow.status == EscrowStatus::Funded,
        EscrowError::InvalidEscrowState
    );

    let idx = index as usize;
    require!(
        idx < ctx.accounts.escrow.milestone_count as usize,
        EscrowError::InvalidMilestoneIndex
    );
    // I3: must be Pending (no re-submit, no submit after terminal).
    require!(
        ctx.accounts.escrow.milestones[idx].status == MilestoneStatus::Pending,
        EscrowError::InvalidMilestoneState
    );
    // In-order: all prior milestones must have left Pending.
    for m in ctx.accounts.escrow.milestones.iter().take(idx) {
        require!(
            !matches!(m.status, MilestoneStatus::Pending),
            EscrowError::OutOfOrderSubmit
        );
    }

    let now = Clock::get()?.unix_timestamp;
    let escrow_key = ctx.accounts.escrow.key();
    let escrow = &mut ctx.accounts.escrow;
    escrow.milestones[idx].status = MilestoneStatus::Submitted;
    escrow.milestones[idx].submitted_at = now;
    escrow.milestones[idx].evidence_hash = evidence_hash;

    emit!(MilestoneSubmitted {
        escrow: escrow_key,
        index,
        evidence_hash,
    });
    Ok(())
}
