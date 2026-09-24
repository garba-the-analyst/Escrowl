//! update_config: admin maintenance — rotate fee/treasury, append allowlist.
//! Open escrows are unaffected (fee snapshot at creation, I5); only future
//! escrows see new values. Pause state is never touched here (see set_paused).
use anchor_lang::prelude::*;

use crate::constants::{CONFIG_SEED, MAX_ALLOWED_MINTS};
use crate::errors::EscrowError;
use crate::events::ConfigUpdated;
use crate::state::Config;
use crate::utils::assert_valid_fee;

#[derive(Accounts)]
pub struct UpdateConfig<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin,
        realloc = Config::max_space(),
        realloc::payer = admin,
        realloc::zero = false,
    )]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<UpdateConfig>,
    new_fee_bps: Option<u16>,
    new_treasury: Option<Pubkey>,
    add_mint: Option<Pubkey>,
) -> Result<()> {
    let config = &mut ctx.accounts.config;

    if let Some(fee) = new_fee_bps {
        assert_valid_fee(fee)?;
        config.fee_bps = fee;
    }
    if let Some(treasury) = new_treasury {
        config.treasury = treasury;
    }
    if let Some(mint) = add_mint {
        require!(
            !config.allowed_mints.contains(&mint),
            EscrowError::AllowlistUpdateInvalid
        );
        require!(
            config.allowed_mints.len() < MAX_ALLOWED_MINTS,
            EscrowError::AllowlistUpdateInvalid
        );
        config.allowed_mints.push(mint);
    }

    emit!(ConfigUpdated {
        fee_bps: config.fee_bps,
        treasury: config.treasury,
        allowlist_len: config.allowed_mints.len() as u8,
    });
    Ok(())
}
