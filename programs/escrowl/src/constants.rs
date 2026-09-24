//! Program-wide constants.

/// Seed for global Config PDA: `["config"]`.
pub const CONFIG_SEED: &[u8] = b"config";
/// Seed for Escrow PDA: `["escrow", buyer, seller, escrow_id_le]`.
pub const ESCROW_SEED: &[u8] = b"escrow";
/// Seed for Vault token account PDA: `["vault", escrow]`.
pub const VAULT_SEED: &[u8] = b"vault";

/// Maximum milestones per escrow (fixed array, keeps account size bounded).
pub const MAX_MILESTONES: usize = 8;
/// Maximum fee in basis points (500 = 5%).
pub const MAX_FEE_BPS: u16 = 500;
/// Basis points denominator.
pub const BPS_DENOMINATOR: u64 = 10_000;
/// Maximum allowed mints in Config allowlist.
pub const MAX_ALLOWED_MINTS: usize = 4;
/// Minimum review window (1 hour) to prevent instant-timeout abuse.
pub const MIN_REVIEW_WINDOW_SECS: i64 = 3_600;
/// Maximum review window (30 days).
pub const MAX_REVIEW_WINDOW_SECS: i64 = 2_592_000;
/// Minimum seller-inactivity deadline (1 day): buyer waits at least this long
/// for a stale milestone before reclaiming (liveness exit, I11).
pub const MIN_SELLER_DEADLINE_SECS: i64 = 86_400;
/// Maximum seller-inactivity deadline (90 days).
pub const MAX_SELLER_DEADLINE_SECS: i64 = 7_776_000;
/// Minimum arbiter timeout (7 days): disputed funds unlock after this long
/// without resolution (liveness exit, I11).
pub const MIN_ARBITER_TIMEOUT_SECS: i64 = 604_800;
/// Maximum arbiter timeout (90 days).
pub const MAX_ARBITER_TIMEOUT_SECS: i64 = 7_776_000;
