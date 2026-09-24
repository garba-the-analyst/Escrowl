//! reclaim_stale_milestone: buyer refunds a Pending milestone the seller
//! abandoned, plus all later Pending milestones (liveness exit, I11).
//!
//! A milestone is reclaimable when it is Pending, every prior milestone is
//! terminal (it is the "active" one), and now >= active_since + seller
//! deadline, where active_since is funded_at for index 0 and the previous
//! milestone's terminal_at otherwise. Refund carries no fee: nothing was
//! delivered. Later Pending milestones are cancelled along with it, so one
//! call unwinds the stalled remainder. Escrow -> Completed when all terminal.
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

use crate::constants::ESCROW_SEED;
use crate::errors::EscrowError;
use crate::events::MilestoneReclaimed;
use crate::state::{Escrow, EscrowStatus, MilestoneStatus};

#[derive(Accounts)]
pub struct ReclaimStaleMilestone<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut, has_one = buyer, has_one = vault, has_one = mint)]
    pub escrow: Account<'info, Escrow>,
    #[account(
        mut,
        constraint = vault.mint == escrow.mint @ EscrowError::MintNotAllowed,
        constraint = vault.owner == escrow.key() @ EscrowError::Unauthorized,
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = buyer_ata.mint == escrow.mint @ EscrowError::MintNotAllowed,
        constraint = buyer_ata.owner == buyer.key() @ EscrowError::Unauthorized,
    )]
    pub buyer_ata: Account<'info, TokenAccount>,
    /// CHECK: mint address must equal escrow terms.
    pub mint: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ReclaimStaleMilestone>, index: u8) -> Result<()> {
    // I9: reclaim is an exit — never gated by pause.
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
        ctx.accounts.escrow.milestones[idx].status == MilestoneStatus::Pending,
        EscrowError::InvalidMilestoneState
    );
    // Only the active milestone (all prior terminal) can go stale.
    for m in ctx.accounts.escrow.milestones.iter().take(idx) {
        require!(m.is_terminal(), EscrowError::InvalidMilestoneState);
    }

    let now = Clock::get()?.unix_timestamp;
    let active_since = ctx
        .accounts
        .escrow
        .milestone_active_since(idx)
        .ok_or(error!(EscrowError::InvalidMilestoneState))?;
    let deadline = active_since
        .checked_add(ctx.accounts.escrow.seller_deadline_secs)
        .ok_or(error!(EscrowError::MathOverflow))?;
    // Boundary: reclaim allowed at exactly the deadline.
    require!(now >= deadline, EscrowError::SellerDeadlineNotElapsed);

    // Sum this and all later Pending milestones (later ones can only be
    // Pending: prior-to-idx are terminal, idx is Pending, Submitted/Disputed
    // later would violate in-order submission... but a later milestone COULD
    // be Submitted only if idx.. were submitted in order, which requires idx
    // non-Pending. Since idx is Pending, all later are Pending. Enforce it.)
    let mut refund: u64 = 0;
    let mut count: u8 = 0;
    for m in ctx.accounts.escrow.milestones.iter().skip(idx) {
        require!(
            matches!(m.status, MilestoneStatus::Pending),
            EscrowError::InvalidMilestoneState
        );
        refund = refund
            .checked_add(m.amount)
            .ok_or(error!(EscrowError::MathOverflow))?;
        count = count
            .checked_add(1)
            .ok_or(error!(EscrowError::MathOverflow))?;
    }
    require!(refund > 0, EscrowError::ZeroMilestoneAmount);

    // Refund vault -> buyer with PDA signer (no fee: nothing delivered).
    let buyer_key = ctx.accounts.escrow.buyer;
    let seller_key = ctx.accounts.escrow.seller;
    let escrow_id = ctx.accounts.escrow.escrow_id;
    let bump = ctx.accounts.escrow.bump;
    let seeds: &[&[u8]] = &[
        ESCROW_SEED,
        buyer_key.as_ref(),
        seller_key.as_ref(),
        &escrow_id.to_le_bytes(),
        &[bump],
    ];
    let signer_seeds = &[seeds];
    let cpi = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::Transfer {
            from: ctx.accounts.vault.to_account_info(),
            to: ctx.accounts.buyer_ata.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer_seeds,
    );
    token::transfer(cpi, refund)?;

    let escrow_key = ctx.accounts.escrow.key();
    let escrow = &mut ctx.accounts.escrow;
    for m in escrow.milestones.iter_mut().skip(idx) {
        m.status = MilestoneStatus::Refunded;
        m.terminal_at = now;
    }
    escrow.refunded_amount = escrow
        .refunded_amount
        .checked_add(refund)
        .ok_or(error!(EscrowError::MathOverflow))?;
    if escrow.all_terminal() {
        escrow.status = EscrowStatus::Completed;
    }

    emit!(MilestoneReclaimed {
        escrow: escrow_key,
        from_index: index,
        milestone_count: count,
        refunded_amount: refund,
    });
    Ok(())
}
