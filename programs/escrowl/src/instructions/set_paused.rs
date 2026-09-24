//! set_paused: admin-only. Pauses create/fund, never exits.
use anchor_lang::prelude::*;

use crate::constants::CONFIG_SEED;
use crate::events::PausedUpdated;
use crate::state::Config;

#[derive(Accounts)]
pub struct SetPaused<'info> {
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [CONFIG_SEED],
        bump = config.bump,
        has_one = admin,
    )]
    pub config: Account<'info, Config>,
}

pub fn handler(ctx: Context<SetPaused>, paused: bool) -> Result<()> {
    ctx.accounts.config.paused = paused;
    emit!(PausedUpdated { paused });
    Ok(())
}
