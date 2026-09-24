//! fund_escrow: buyer transfers exact total into vault. Once only.
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

use crate::constants::CONFIG_SEED;
use crate::errors::EscrowError;
use crate::events::EscrowFunded;
use crate::state::{Config, Escrow, EscrowStatus};

#[derive(Accounts)]
pub struct FundEscrow<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(
        mut,
        has_one = buyer,
        has_one = vault,
        has_one = mint,
    )]
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
    /// Mint must match escrow terms.
    /// CHECK: validated via has_one + token account mints; address only.
    pub mint: UncheckedAccount<'info>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
    pub token_program: Program<'info, Token>,
}

pub fn handler(ctx: Context<FundEscrow>) -> Result<()> {
    require!(!ctx.accounts.config.paused, EscrowError::Paused);
    require!(
        ctx.accounts.escrow.status == EscrowStatus::Created,
        EscrowError::InvalidEscrowState
    );

    let total = ctx.accounts.escrow.total_amount;
    require!(total > 0, EscrowError::ZeroMilestoneAmount);
    // Vault must be empty before first and only funding (prevents double-fund).
    require!(
        ctx.accounts.vault.amount == 0,
        EscrowError::InvalidEscrowState
    );

    // Transfer exact total buyer -> vault.
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        token::Transfer {
            from: ctx.accounts.buyer_ata.to_account_info(),
            to: ctx.accounts.vault.to_account_info(),
            authority: ctx.accounts.buyer.to_account_info(),
        },
    );
    token::transfer(cpi_ctx, total)?;

    // I1 holds after funding: vault == total (all milestones Pending => locked == total).
    ctx.accounts.vault.reload()?;
    require!(
        ctx.accounts.vault.amount == total,
        EscrowError::VaultBalanceMismatch
    );
    require!(
        ctx.accounts.vault.amount == ctx.accounts.escrow.locked_amount()?,
        EscrowError::VaultBalanceMismatch
    );

    ctx.accounts.escrow.status = EscrowStatus::Funded;
    // Starts milestone 0's seller-inactivity clock (liveness, I11).
    ctx.accounts.escrow.funded_at = Clock::get()?.unix_timestamp;

    emit!(EscrowFunded {
        escrow: ctx.accounts.escrow.key(),
        amount: total,
    });
    Ok(())
}
