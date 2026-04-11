# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **SLSA Build Level 2 provenance** — release artifacts are attested via
  `actions/attest-build-provenance` and verified with
  `gh attestation verify <file> --repo DracoWhitefire/concordance`.

### Internal

- Fixed coverage ratchet CI: added `LC_NUMERIC=C` to the baseline `printf` to prevent
  locale-dependent decimal separators from corrupting `.coverage-baseline` on non-C locales.

## [0.1.0]

Initial release.

### Added

- Three-stage negotiation pipeline: enumerate candidates, check constraints, rank results
- `is_config_viable` — no-alloc constraint probe for firmware and embedded targets
- `NegotiatorBuilder` — ranked pipeline for `alloc`/`std` targets
- `DefaultConstraintEngine` — HDMI 2.1 specification constraint checks
- `DefaultEnumerator` and `SliceEnumerator` — Cartesian product candidate generation
- `DefaultRanker` with `NegotiationPolicy` presets (`BestQuality`, `BestPerformance`,
  `PowerSaving`)
- `ReasoningTrace` — per-config audit log of constraint decisions and ranking criteria
- `ConstraintRule` trait and `Layered` combinator for additive rule injection
- `sink_capabilities_from_display` — bridge from `DisplayCapabilities` (piaf) to
  `SinkCapabilities`
- `CableCapabilities::unconstrained()` — optimistic fallback for callers without cable
  information
- `serde` feature flag: `Serialize`/`Deserialize` on all public types
- `no_std` support at all three resource tiers (no-alloc, alloc, std)

### Internal

- Coverage ratchet: CI measures line coverage across the `std` and `std + serde` feature
  sets using `cargo-llvm-cov`. The baseline is stored in `.coverage-baseline`; CI fails if
  coverage drops more than 0.1% below it. On pushes to `main` or `develop`, coverage
  improvements are committed automatically via a `ci/coverage-ratchet` PR.
