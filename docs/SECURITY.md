# Security Policy

## Reporting

Do not open public issues for vulnerabilities. Contact the maintainer privately
with: affected version/commit, reproduction steps, impact assessment.

We aim to acknowledge within 48h and patch devnet deployments promptly.

## Self-audit (Phase 8 checklist)

- [x] `cargo clippy --all-targets --locked` clean (workspace lints deny
  `clippy::all` + `unwrap_used`/`expect_used`, forbid `unsafe_code`)
  — verified 2026-09-24, 0 errors.
- [x] `cargo fmt --check` clean — verified 2026-09-24.
- [ ] `cargo audit` clean
- [ ] `cargo deny check` clean
- [ ] `cargo geiger` reviewed (no unexpected unsafe)
- [x] Rust tests: 15 unit + 3 model-fuzz (`cargo test -p escrowl --locked`,
  18/18 green) + 19 Anchor integration cases (`anchor test`, all passing).
  Findings in docs/FUZZ_FINDINGS.md.
- [ ] Trident on-chain fuzz (see `tests/fuzz/README.md` for the runbook).
- [ ] Verifiable build via `solana-verify`.

## Guarantees

- No `unwrap()` / `expect()` in program code.
- All arithmetic `checked_*` mapped to custom errors.
- SPL Token classic only; Token-2022 rejected.
- Pause never blocks user exits.
