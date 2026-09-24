//! Global Config PDA: `["config"]`.

use anchor_lang::prelude::*;

use crate::constants::MAX_ALLOWED_MINTS;

/// Global protocol config. Admin-controlled.
#[account]
pub struct Config {
    /// Admin authority (can update fee, pause, allowlist).
    pub admin: Pubkey,
    /// Treasury token account owner receiving fees.
    pub treasury: Pubkey,
    /// Protocol fee in basis points, snapshot per escrow at creation.
    pub fee_bps: u16,
    /// When true, blocks create/fund only. Exits never blocked.
    pub paused: bool,
    /// SPL classic mint allowlist (USDC devnet etc).
    pub allowed_mints: Vec<Pubkey>,
    /// Canonical bump.
    pub bump: u8,
}

impl Config {
    pub fn space(num_mints: usize) -> usize {
        8 + 32 + 32 + 2 + 1 + (4 + num_mints * 32) + 1
    }

    pub fn max_space() -> usize {
        Self::space(MAX_ALLOWED_MINTS)
    }
}
