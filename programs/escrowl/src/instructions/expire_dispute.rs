//! expire_dispute: either party unlocks a Disputed milestone the arbiter
//! abandoned, splitting 50/50 after the arbiter timeout (liveness exit, I11).
//!
//! Requires Disputed status and now >= disputed_at + arbiter_timeout_secs.
//! Split: seller leg = floor(amount / 2) with the protocol fee applied exactly
//! as in resolve_dispute; buyer receives the remainder (so the odd unit goes
//! to the buyer). Marks Resolved. Either buyer or seller may call: the signer
//! must equal one of the two stored counterparties.
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

use crate::constants::ESCROW_SEED;
use crate::errors::EscrowError;
use crate::events::DisputeExpired;
use crate::state::{Escrow, EscrowStatus, MilestoneStatus};
use crate::utils::{calc_fee, calc_seller_amount};

#[derive(Accounts)]
pub struct ExpireDispute<'info> {
    /// Either counterparty; checked against stored buyer/seller in handler.
    #[account(mut)]
    pub authority: Signer<'info>,
    /// Buyer and seller pubkeys (need not sign; compared to authority).
    /// CHECK: validated against stored roles; no data read.
    pub buyer: UncheckedAccount<'info>,
    /// CHECK: validated against stored roles; no data read.
    pub seller: UncheckedAccount<'info>,
    #[account(mut, has_one = vault, has_one = mint)]
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

pub fn handler(ctx: Context<ExpireDispute>, index: u8) -> Result<()> {
    // I9: expiry is an exit — never gated by pause.
    require!(
        ctx.accounts.escrow.status == EscrowStatus::Funded,
        EscrowError::InvalidEscrowState
    );
    // I2 adapted: caller must be one of the two counterparties (never the
    // arbiter — this exit exists precisely for arbiter absence).
    let authority_key = ctx.accounts.authority.key();
    require!(
        authority_key == ctx.accounts.escrow.buyer || authority_key == ctx.accounts.escrow.seller,
        EscrowError::Unauthorized
    );
    // The passed buyer/seller accounts must match stored roles (prevents
    // confusing indexers and keeps the interface explicit).
    require!(
        ctx.accounts.buyer.key() == ctx.accounts.escrow.buyer
            && ctx.accounts.seller.key() == ctx.accounts.escrow.seller,
        EscrowError::Unauthorized
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

    let now = Clock::get()?.unix_timestamp;
    let disputed_at = ctx.accounts.escrow.milestones[idx].disputed_at;
    let timeout_end = disputed_at
        .checked_add(ctx.accounts.escrow.arbiter_timeout_secs)
        .ok_or(error!(EscrowError::MathOverflow))?;
    // Boundary: expiry allowed at exactly the timeout.
    require!(now >= timeout_end, EscrowError::DisputeNotExpired);

    // 50/50 split; odd unit goes to the buyer. Fee on the seller leg only,
    // identical to resolve_dispute accounting.
    let amount = ctx.accounts.escrow.milestones[idx].amount;
    let seller_leg = amount
        .checked_div(2)
        .ok_or(error!(EscrowError::MathOverflow))?;
    let buyer_leg = amount
        .checked_sub(seller_leg)
        .ok_or(error!(EscrowError::MathOverflow))?;
    // Conservation oracle for the split (fails closed on any violation).
    require!(
        seller_leg
            .checked_add(buyer_leg)
            .ok_or(error!(EscrowError::MathOverflow))?
            == amount,
        EscrowError::MathOverflow
    );
    let fee_bps = ctx.accounts.escrow.fee_bps_snapshot;
    let fee = calc_fee(seller_leg, fee_bps)?;
    let seller_proceeds = calc_seller_amount(seller_leg, fee_bps)?;

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
    if buyer_leg > 0 {
        let cpi = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.vault.to_account_info(),
                to: ctx.accounts.buyer_ata.to_account_info(),
                authority: ctx.accounts.escrow.to_account_info(),
            },
            signer_seeds,
        );
        token::transfer(cpi, buyer_leg)?;
    }

    let escrow_key = ctx.accounts.escrow.key();
    let escrow = &mut ctx.accounts.escrow;
    escrow.milestones[idx].status = MilestoneStatus::Resolved;
    escrow.milestones[idx].terminal_at = now;
    escrow.released_amount = escrow
        .released_amount
        .checked_add(seller_leg)
        .ok_or(error!(EscrowError::MathOverflow))?;
    escrow.refunded_amount = escrow
        .refunded_amount
        .checked_add(buyer_leg)
        .ok_or(error!(EscrowError::MathOverflow))?;
    if escrow.all_terminal() {
        escrow.status = EscrowStatus::Completed;
    }

    emit!(DisputeExpired {
        escrow: escrow_key,
        index,
        seller_amount: seller_leg,
        buyer_amount: buyer_leg,
    });
    Ok(())
}
