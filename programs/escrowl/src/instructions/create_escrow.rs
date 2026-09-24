//! create_escrow: buyer creates terms + PDA vault.
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::constants::{CONFIG_SEED, ESCROW_SEED, VAULT_SEED};
use crate::events::EscrowCreated;
use crate::state::{Config, Escrow, EscrowStatus, Milestone, MilestoneStatus};
use crate::utils::{
    assert_distinct_roles, assert_valid_allowlist, assert_valid_arbiter_timeout,
    assert_valid_milestones, assert_valid_seller_deadline, assert_valid_window, checked_sum,
};

#[derive(Accounts)]
#[instruction(escrow_id: u64)]
pub struct CreateEscrow<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    /// Seller pubkey (need not sign at creation).
    /// CHECK: validated distinct + stored; no data read.
    pub seller: UncheckedAccount<'info>,
    /// Arbiter pubkey (need not sign at creation).
    /// CHECK: validated distinct + stored.
    pub arbiter: UncheckedAccount<'info>,
    /// SPL classic mint; must be in allowlist. Token-2022 rejected in handler.
    pub mint: Account<'info, Mint>,
    #[account(seeds = [CONFIG_SEED], bump = config.bump)]
    pub config: Account<'info, Config>,
    #[account(
        init,
        payer = buyer,
        space = 8 + Escrow::INIT_SPACE,
        seeds = [ESCROW_SEED, buyer.key().as_ref(), seller.key().as_ref(), &escrow_id.to_le_bytes()],
        bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        init,
        payer = buyer,
        seeds = [VAULT_SEED, escrow.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = escrow,
    )]
    pub vault: Account<'info, TokenAccount>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handler(
    ctx: Context<CreateEscrow>,
    escrow_id: u64,
    milestone_amounts: Vec<u64>,
    review_window_secs: i64,
    seller_deadline_secs: i64,
    arbiter_timeout_secs: i64,
) -> Result<()> {
    let config = &ctx.accounts.config;

    // I9: pause blocks creation.
    require!(!config.paused, crate::errors::EscrowError::Paused);

    let buyer_key = ctx.accounts.buyer.key();
    let seller_key = ctx.accounts.seller.key();
    let arbiter_key = ctx.accounts.arbiter.key();

    // I6: pairwise distinct roles.
    assert_distinct_roles(&buyer_key, &seller_key, &arbiter_key)?;
    // I7: count 1..=8, each > 0.
    assert_valid_milestones(&milestone_amounts)?;
    assert_valid_window(review_window_secs)?;
    // I11: liveness windows bounded (1d..90d seller, 7d..90d arbiter).
    assert_valid_seller_deadline(seller_deadline_secs)?;
    assert_valid_arbiter_timeout(arbiter_timeout_secs)?;
    assert_valid_allowlist(&config.allowed_mints)?;

    // I8: mint must be allowlisted...
    require!(
        config
            .allowed_mints
            .contains(ctx.accounts.mint.to_account_info().key),
        crate::errors::EscrowError::MintNotAllowed
    );
    // ...and owned by the classic SPL Token program (reject Token-2022).
    require!(
        *ctx.accounts.mint.to_account_info().owner == anchor_spl::token::ID,
        crate::errors::EscrowError::Token2022Rejected
    );
    require!(
        ctx.accounts.token_program.key() == anchor_spl::token::ID,
        crate::errors::EscrowError::Token2022Rejected
    );

    // I7: total == sum(parts), checked.
    let total = checked_sum(&milestone_amounts)?;
    require!(total > 0, crate::errors::EscrowError::ZeroMilestoneAmount);

    let milestones: Vec<Milestone> = milestone_amounts
        .iter()
        .map(|a| Milestone {
            amount: *a,
            status: MilestoneStatus::Pending,
            submitted_at: 0,
            terminal_at: 0,
            disputed_at: 0,
            evidence_hash: [0u8; 32],
        })
        .collect();

    let milestone_count = u8::try_from(milestones.len())
        .map_err(|_| error!(crate::errors::EscrowError::InvalidMilestoneCount))?;
    let clock = Clock::get()?;

    let escrow = &mut ctx.accounts.escrow;
    escrow.buyer = buyer_key;
    escrow.seller = seller_key;
    escrow.arbiter = arbiter_key;
    escrow.mint = ctx.accounts.mint.key();
    escrow.vault = ctx.accounts.vault.key();
    escrow.escrow_id = escrow_id;
    escrow.milestone_count = milestone_count;
    escrow.milestones = milestones;
    escrow.total_amount = total;
    escrow.released_amount = 0;
    escrow.refunded_amount = 0;
    // I5: snapshot fee AND treasury so later Config changes don't affect
    // open escrows (treasury rotation only redirects future escrows' fees).
    escrow.fee_bps_snapshot = config.fee_bps;
    escrow.treasury_snapshot = config.treasury;
    escrow.review_window_secs = review_window_secs;
    escrow.seller_deadline_secs = seller_deadline_secs;
    escrow.arbiter_timeout_secs = arbiter_timeout_secs;
    escrow.status = EscrowStatus::Created;
    escrow.created_at = clock.unix_timestamp;
    escrow.funded_at = 0; // set once by fund_escrow; starts milestone 0's clock
    escrow.bump = ctx.bumps.escrow;
    escrow.vault_bump = ctx.bumps.vault;

    emit!(EscrowCreated {
        escrow: escrow.key(),
        buyer: buyer_key,
        seller: seller_key,
        arbiter: arbiter_key,
        total_amount: total,
        milestone_count,
    });
    Ok(())
}
