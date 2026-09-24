# Decisions

Log safest-interpretation choices here when spec is ambiguous.

| Date | Decision | Rationale |
|---|---|---|
| 2026-09-23 | SPL classic only, reject Token-2022 | Removes transfer-hook / fee-extension audit surface for MVP |
| 2026-09-23 | Pause blocks create/fund only | Users must always be able to exit; prevents admin trap |
| 2026-09-23 | Fee snapshot at creation | Later Config changes don't retroactively affect open escrows |
| 2026-09-23 | Single arbiter, buyer-only disputes | Keeps scope buildable in 19 days; DAO voting = roadmap |
| 2026-09-23 | Toolchain pinned to Rust 1.89.0 (was 1.84.0) | Transitive deps via anchor-spl now require edition2024 (stabilized 1.85); 1.84 fails resolution. Verified with `cargo check` on 1.97 + Cargo.lock committed |
| 2026-09-23 | SDK IDL hand-authored (`sdk/idl/escrowl.json` via script) | No Anchor CLI in this env; discriminators are real sighashes (verified vs sha256). MUST diff against `anchor build` output on dev machine before shipping — see `sdk/idl/README.md` |
| 2026-09-24 | Added admin `update_config` (fee/treasury rotation, allowlist append, new error 6022) | Immutable allowlist blocked test suites sharing the singleton config AND is a launch-readiness gap (can't list USDC later without redeploy). Open escrows unaffected (fee snapshot). Realloc capped at max space |
| 2026-09-24 | Test ATAs use idempotent creation | Re-creating an ATA for the same owner+mint fails on-chain ("Provided owner is not allowed"); suites share buyer keypairs across escrows, so idempotent is required |
| 2026-09-24 | Suites read treasury/mint from singleton config | 02/03 must use config.treasury for treasury ATAs and onboard own mints via update_config; wrong-owner ATAs fail constraint checks by design |
| 2026-09-24 | Clippy via workspace lints, not `-D warnings` | `-D warnings` elevates `missing_docs` (~150 items) + anchor-macro `unexpected_cfgs` to hard errors. Policy lives in `[workspace.lints]` (clippy::all + unwrap/expect deny, unsafe forbid); fixed 3 real denies newer clippy found (test unwraps, manual range check, derivable Defaults) |
| 2026-09-24 | Frontend: Next 15.1.12 + React 19, SDK via `file:` link | Next 15.0.0 peer-rejects React 19 stable (ERESOLVE) and has CVE-2025-66478; 15.1 accepts `^19.0.0`. `@escrowl/sdk` linked with `transpilePackages` (relative `../../sdk` depths were wrong per page). All web3 value imports from `@solana/web3.js` — anchor 0.31 re-exports only BN/Program/AnchorProvider |
| 2026-09-23 | LiteSVM dev-dependency dropped for Phase 2 | `litesvm 0.8` pulls Solana 3.x + crates needing Rust 1.87+; TS tests cover Phase 3, fuzz harness re-adds a pinned version in Phase 4 |
| 2026-09-23 | Glob re-exports in instructions/mod.rs kept | Anchor 0.31 `#[program]` macro resolves account types through them; removal breaks compile. `ambiguous_glob_reexports` allowed with rationale comment |
