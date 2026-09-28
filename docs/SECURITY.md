# Security Policy

## Reporting

Do not open public issues for vulnerabilities. Contact the maintainer privately
with: affected version/commit, reproduction steps, impact assessment.

> <!-- VERIFY --> Fill in a real disclosure contact before the audit
> application (email or Telegram handle).

We aim to acknowledge within 48h and patch devnet deployments promptly.

## Self-audit (Phase 8 checklist)

- [x] `cargo clippy --all-targets --locked` clean (workspace lints deny
  `clippy::all` + `unwrap_used`/`expect_used`, forbid `unsafe_code`)
  — verified 2026-09-24, 0 errors.
- [x] `cargo fmt --check` clean — verified 2026-09-24.
- [x] `cargo audit` clean (exit 0; 3 allowed warnings, no vulnerabilities)
  — verified 2026-09-24.
- [x] `cargo deny check` clean (advisories/bans/licenses/sources ok;
  2 narrowly-scoped unmaintained ignores with rationale in `deny.toml`)
  — verified 2026-09-24.
- [ ] `cargo geiger` reviewed (no unexpected unsafe)
- [x] Rust tests: 16 unit + 3 model checks (`cargo test -p escrowl --locked`)
  + 19 Anchor integration (`anchor test`) + 23 LiteSVM real-program tests.
  Findings in docs/FUZZ_FINDINGS.md.
- [x] Reproducible build: `solana-verify build` succeeds deterministically
  (hashes in docs/DEPLOYMENT.md). On-chain binary sync pending next funded
  redeploy.
- [ ] Trident on-chain fuzz (see `tests/fuzz/README.md` for the runbook).

## Guarantees

- No `unwrap()` / `expect()` in program code.
- All arithmetic `checked_*` mapped to custom errors.
- SPL Token classic only; Token-2022 rejected.
- Pause never blocks user exits.

## Known limitations

- Arbiter is a trusted role per escrow (see Trust assumptions in
  docs/THREAT_MODEL.md).
- Single admin key pre-mainnet (pause, fee/treasury rotation, allowlist).
- No Token-2022 support (rejected by design).

## Upgrade authority

Devnet program is upgradeable by the deploy keypair (gitignored at
`target/deploy/escrowl-keypair.json` — back it up offline; losing it loses
the program ID). Mainnet recommendation: squad multisig + timelock, with a
plan to renounce upgrades after audit.
