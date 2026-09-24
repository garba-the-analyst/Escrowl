//! resolve_dispute: arbiter splits exactly. Fee on seller leg only.
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

use crate::constants::ESCROW_SEED;
use crate::errors::EscrowError;
use crate::events::DisputeResolved;
use crate::state::{Escrow, EscrowStatus, MilestoneStatus};
use crate::utils::{calc_fee, calc_seller_amount};

#[derive(Accounts)]
pub struct ResolveDispute<'info> {
    #[account(mut)]
    pub arbiter: Signer<'info>,
    #[account(mut, has_one = arbiter, has_one = vault, has_one = mint)]
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
        constraint = buyer_ata.mint == escrow.mint @ EscrowError::MintNotAllowed,
        constraint = buyer_ata.owner == escrow.buyer @ EscrowError::Unauthorized,
    )]
    pub buyer_ata: Account<'info, TokenAccount>,
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

pub fn handler(
    ctx: Context<ResolveDispute>,
    index: u8,
    seller_amount: u64,
    buyer_amount: u64,
) -> Result<()> {
    // I9: resolution is an exit — never gated by pause.
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
        ctx.accounts.escrow.milestones[idx].status == MilestoneStatus::Disputed,
        EscrowError::InvalidMilestoneState
    );

    // I4: split must sum exactly to milestone amount (checked, no dust creation).
    let milestone_amount = ctx.accounts.escrow.milestones[idx].amount;
    let split_total = seller_amount
        .checked_add(buyer_amount)
        .ok_or(error!(EscrowError::MathOverflow))?;
    require!(
        split_total == milestone_amount,
        EscrowError::InvalidSplitSum
    );

    // I5: fee applies to seller leg only; buyer refund is whole.
    let fee_bps = ctx.accounts.escrow.fee_bps_snapshot;
    let fee = calc_fee(seller_amount, fee_bps)?;
    let seller_proceeds = calc_seller_amount(seller_amount, fee_bps)?;

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

    // Order: seller, treasury, buyer. Skip zero-amount CPIs.
    if seller_proceeds > 0 {
        let cpi = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.seller_ata.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi, seller_proceeds)?;
    }
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
    if buyer_amount > 0 {
        let cpi = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.buyer_ata.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi, buyer_amount)?;
    }

    let now = Clock::get()?.unix_timestamp;
    let escrow_key = ctx.accounts.escrow.key();
    let escrow = &mut ctx.accounts.escrow;
    // I3: Disputed -> Resolved exactly once.
    escrow.milestones[idx].status = MilestoneStatus::Resolved;
    escrow.milestones[idx].terminal_at = now;
    escrow.released_amount = escrow
        .released_amount
        .checked_add(seller_amount)
        .ok_or(error!(EscrowError::MathOverflow))?;
    escrow.refunded_amount = escrow
        .refunded_amount
        .checked_add(buyer_amount)
        .ok_or(error!(EscrowError::MathOverflow))?;
    if escrow.all_terminal() {
        escrow.status = EscrowStatus::Completed;
    }

    emit!(DisputeResolved {
        escrow: escrow_key,
        index,
        seller_amount,
        buyer_amount,
    });
    Ok(())
}
