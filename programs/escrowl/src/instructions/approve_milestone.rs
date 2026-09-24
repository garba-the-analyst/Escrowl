//! approve_milestone: buyer releases seller + treasury fee split.
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

use crate::constants::ESCROW_SEED;
use crate::errors::EscrowError;
use crate::events::{MilestoneReleased, ReleaseReason};
use crate::state::{Escrow, EscrowStatus, MilestoneStatus};
use crate::utils::{calc_fee, calc_seller_amount};

#[derive(Accounts)]
pub struct ApproveMilestone<'info> {
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
        constraint = seller_ata.mint == escrow.mint @ EscrowError::MintNotAllowed,
        constraint = seller_ata.owner == escrow.seller @ EscrowError::Unauthorized,
    )]
    pub seller_ata: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = treasury_ata.mint == escrow.mint @ EscrowError::MintNotAllowed,
        constraint = treasury_ata.owner == escrow.treasury_snapshot @ EscrowError::Unauthorized,
    )]
    pub treasury_ata: Account<'info, TokenAccount>,
    /// CHECK: mint address must equal escrow terms.
    pub mint: UncheckedAccount<'info>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<ApproveMilestone>, index: u8) -> Result<()> {
    // I9: exits always allowed — no paused check here by design.
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

    // I2: has_one = buyer already enforces buyer-only (signer + stored).
    let amount = ctx.accounts.escrow.milestones[idx].amount;
    let fee_bps = ctx.accounts.escrow.fee_bps_snapshot;
    let fee = calc_fee(amount, fee_bps)?;
    let seller_amount = calc_seller_amount(amount, fee_bps)?;

    // PDA signer for vault authority = escrow.
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

    // Pay seller (dust-inclusive remainder).
    if seller_amount > 0 {
        let cpi = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.seller_ata.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi, seller_amount)?;
    }
    // Pay treasury fee (skip zero to save CU + avoid dust issues).
    if fee > 0 {
        let cpi = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.treasury_ata.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi, fee)?;
    }

    // I3: terminal transition exactly once.
    let now = Clock::get()?.unix_timestamp;
    let escrow_key = ctx.accounts.escrow.key();
    let escrow = &mut ctx.accounts.escrow;
    escrow.milestones[idx].status = MilestoneStatus::Released;
    escrow.milestones[idx].terminal_at = now;
    escrow.released_amount = escrow
        .released_amount
        .checked_add(amount)
        .ok_or(error!(EscrowError::MathOverflow))?;
    if escrow.all_terminal() {
        escrow.status = EscrowStatus::Completed;
    }

    emit!(MilestoneReleased {
        escrow: escrow_key,
        index,
        seller_amount,
        fee_amount: fee,
        reason: ReleaseReason::Approved,
    });
    Ok(())
}
