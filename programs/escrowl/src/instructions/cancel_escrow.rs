//! cancel_escrow: buyer refunds if no work started.
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};

use crate::constants::ESCROW_SEED;
use crate::errors::EscrowError;
use crate::events::EscrowCancelled;
use crate::state::{Escrow, EscrowStatus, MilestoneStatus};

#[derive(Accounts)]
pub struct CancelEscrow<'info> {
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

pub fn handler(ctx: Context<CancelEscrow>) -> Result<()> {
    // I9: cancel is an exit — never gated by pause.
    let status = ctx.accounts.escrow.status;
    match status {
        EscrowStatus::Created => {
            // No funds moved yet; vault must be empty.
            require!(ctx.accounts.vault.amount == 0, EscrowError::VaultNotEmpty);
            let now = Clock::get()?.unix_timestamp;
            let escrow_key = ctx.accounts.escrow.key();
            let escrow = &mut ctx.accounts.escrow;
            // No funds ever moved: mark Cancelled (NOT Refunded) so indexers
            // are not misled, and close sees all-terminal.
            for m in &mut escrow.milestones {
                if matches!(m.status, MilestoneStatus::Pending) {
                    m.status = MilestoneStatus::Cancelled;
                    m.terminal_at = now;
                }
            }
            escrow.status = EscrowStatus::Cancelled;
            emit!(EscrowCancelled { escrow: escrow_key });
            Ok(())
        }
        EscrowStatus::Funded => {
            // Only if zero milestones ever left Pending (no work started).
            require!(
                !ctx.accounts.escrow.any_submitted_or_beyond(),
                EscrowError::CancelNotAllowed
            );
            let total = ctx.accounts.escrow.total_amount;
            ctx.accounts.vault.reload()?;
            require!(
                ctx.accounts.vault.amount == total,
                EscrowError::VaultBalanceMismatch
            );

            // Refund full vault -> buyer with PDA signer.
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

            if total > 0 {
                let cpi = CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    token::Transfer {
                        from: ctx.accounts.vault.to_account_info(),
                        to: ctx.accounts.buyer_ata.to_account_info(),
                        authority: ctx.accounts.escrow.to_account_info(),
                    },
                    signer_seeds,
                );
                token::transfer(cpi, total)?;
            }

            let now = Clock::get()?.unix_timestamp;
            let escrow_key = ctx.accounts.escrow.key();
            let escrow = &mut ctx.accounts.escrow;
            for m in &mut escrow.milestones {
                if matches!(m.status, MilestoneStatus::Pending) {
                    m.status = MilestoneStatus::Refunded;
                    m.terminal_at = now;
                }
            }
            escrow.refunded_amount = escrow
                .refunded_amount
                .checked_add(total)
                .ok_or(error!(EscrowError::MathOverflow))?;
            escrow.status = EscrowStatus::Cancelled;

            emit!(EscrowCancelled { escrow: escrow_key });
            Ok(())
        }
        _ => Err(error!(EscrowError::InvalidEscrowState)),
    }
}
