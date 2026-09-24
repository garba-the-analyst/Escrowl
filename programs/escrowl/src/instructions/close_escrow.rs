//! close_escrow: reclaim rent when all terminal, vault empty.
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

use crate::constants::ESCROW_SEED;
use crate::errors::EscrowError;
use crate::events::EscrowClosed;
use crate::state::{Escrow, EscrowStatus};

#[derive(Accounts)]
pub struct CloseEscrow<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        mut,
        has_one = buyer,
        has_one = vault,
        close = buyer
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        mut,
        constraint = vault.mint == escrow.mint @ EscrowError::MintNotAllowed,
        constraint = vault.owner == escrow.key() @ EscrowError::Unauthorized,
    )]
    pub vault: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<CloseEscrow>) -> Result<()> {
    // I10: single explicit guard — status is Completed or Cancelled.
    // Every path that makes all milestones terminal also sets the status
    // (approve/claim/resolve/reclaim set Completed; cancel sets Cancelled),
    // so no second condition is needed.
    require!(
        matches!(
            ctx.accounts.escrow.status,
            EscrowStatus::Completed | EscrowStatus::Cancelled
        ),
        EscrowError::NotAllTerminal
    );

    ctx.accounts.vault.reload()?;
    require!(ctx.accounts.vault.amount == 0, EscrowError::VaultNotEmpty);

    // Close SPL vault first (authority = escrow PDA), lamports to buyer.
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

    let close_cpi = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::CloseAccount {
            account: ctx.accounts.vault.to_account_info(),
            destination: ctx.accounts.buyer.to_account_info(),
            authority: ctx.accounts.escrow.to_account_info(),
        },
        signer_seeds,
    );
    token::close_account(close_cpi)?;

    // Anchor `close = buyer` reclaims escrow lamports to buyer on exit.
    let escrow_key = ctx.accounts.escrow.key();
    emit!(EscrowClosed { escrow: escrow_key });
    Ok(())
}
