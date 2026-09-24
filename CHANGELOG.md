# Changelog

## Unreleased (toward v0.1.0-hackathon)

### Added
- `reclaim_stale_milestone` (buyer refunds abandoned Pending milestones past
  the seller deadline) and `expire_dispute` (either party unlocks abandoned
  disputes 50/50 past the arbiter timeout) — liveness exits (I11).
- `update_config` (admin fee/treasury rotation, allowlist append).
- `treasury_snapshot` per escrow; `funded_at` / `terminal_at` / `disputed_at`
  timestamps; `seller_deadline_secs` / `arbiter_timeout_secs` per escrow.
- `Cancelled` milestone status (Created-cancel marks Cancelled, not Refunded).
- LiteSVM real-program suites: lifecycle, liveness, adversarial (incl.
  aliasing), 200x100 randomized conservation fuzz.
- `docs/AUDIT_PREP.md`, `docs/SELF_AUDIT.md`, `scripts/verify-repo.sh`,
  `scripts/setup-toolchain.sh`, `scripts/status-devnet.ts`.

### Changed
- Removed `NotImplemented` error; codes renumbered from 6000 (see errors.rs).
- `locked_amount()` uses checked math (Result).
- `VaultInvariantViolated` split into `VaultBalanceMismatch`/`VaultNotEmpty`.
- `close_escrow` guard simplified to status Completed|Cancelled.
- Approve/claim/resolve check treasury against the escrow snapshot.

### Fixed
- F-01: I1 scoped to Funded state (vault legitimately empty pre-deposit).

## v0.0.0 — initial devnet deployment (2026-09-24)

- 11-instruction program deployed to devnet at
  `61YPTaqaVeh4dywJEFm21jLaRhHRqeiEiG1gGNox3zwE`; seeded escrows in every
  state. Superseded by Unreleased (requires redeploy + reseed).
