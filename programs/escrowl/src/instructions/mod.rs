//! Instruction handlers, one module per instruction.
//!
//! Glob re-exports are intentional: Anchor's `#[program]` macro resolves
//! account types through these paths. The resulting `handler` name overlap
//! is unused (callers use full module paths).
#![allow(ambiguous_glob_reexports)]

pub mod approve_milestone;
pub mod cancel_escrow;
pub mod claim_after_timeout;
pub mod close_escrow;
pub mod create_escrow;
pub mod expire_dispute;
pub mod fund_escrow;
pub mod initialize_config;
pub mod raise_dispute;
pub mod reclaim_stale_milestone;
pub mod resolve_dispute;
pub mod set_paused;
pub mod submit_milestone;
pub mod update_config;

pub use approve_milestone::*;
pub use cancel_escrow::*;
pub use claim_after_timeout::*;
pub use close_escrow::*;
pub use create_escrow::*;
pub use expire_dispute::*;
pub use fund_escrow::*;
pub use initialize_config::*;
pub use raise_dispute::*;
pub use reclaim_stale_milestone::*;
pub use resolve_dispute::*;
pub use set_paused::*;
pub use submit_milestone::*;
pub use update_config::*;
