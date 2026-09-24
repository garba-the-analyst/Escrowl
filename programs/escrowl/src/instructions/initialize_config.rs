//! initialize_config: create global Config PDA.
use anchor_lang::prelude::*;

use crate::constants::CONFIG_SEED;
use crate::events::ConfigInitialized;
use crate::state::Config;
use crate::utils::{assert_valid_allowlist, assert_valid_fee};

#[derive(Accounts)]
pub struct InitializeConfig<'info> {
    /// Admin authority stored in config (also pays for init).
    #[account(mut)]
    pub admin: Signer<'info>,
    /// Treasury pubkey receiving fees (any account, only pubkey stored).
    /// CHECK: only pubkey is stored; no lamports moved here.
    pub treasury: UncheckedAccount<'info>,
    #[account(
        init,
        payer = admin,
        space = Config::max_space(),
        seeds = [CONFIG_SEED],
        bump
    )]
    pub config: Account<'info, Config>,
    pub system_program: Program<'info, System>,
}

pub fn handler(
    ctx: Context<InitializeConfig>,
    fee_bps: u16,
    allowed_mints: Vec<Pubkey>,
) -> Result<()> {
    assert_valid_fee(fee_bps)?;
    assert_valid_allowlist(&allowed_mints)?;
    let config = &mut ctx.accounts.config;
    config.admin = ctx.accounts.admin.key();
    config.treasury = ctx.accounts.treasury.key();
    config.fee_bps = fee_bps;
    config.paused = false;
    config.allowed_mints = allowed_mints;
    config.bump = ctx.bumps.config;

    emit!(ConfigInitialized {
        admin: config.admin,
        treasury: config.treasury,
        fee_bps,
    });
    Ok(())
}
