# Roadmap

## Shipped

### 0.1.0 — Initial release

Three-stage negotiation pipeline: enumerate candidates, constrain against HDMI 2.1 rules,
rank by policy.

- `is_config_viable` — no-alloc constraint probe for firmware and embedded targets
- `NegotiatorBuilder` — ranked pipeline for `alloc`/`std` targets
- `DefaultConstraintEngine` with HDMI 2.1 specification constraint checks
- `DefaultEnumerator` and `SliceEnumerator` — Cartesian product candidate generation
- `DefaultRanker` with `NegotiationPolicy` and presets (`BestQuality`, `BestPerformance`,
  `PowerSaving`)
- `ReasoningTrace` — per-config audit log of constraint decisions and ranking criteria
- `ConstraintRule` trait and `Layered` combinator for additive rule injection
- `sink_capabilities_from_display` — bridge from `DisplayCapabilities` (piaf) to
  `SinkCapabilities`
- `CableCapabilities::unconstrained()` for callers without cable information
- `serde` feature: `Serialize`/`Deserialize` on all public types
- `no_std` support at all three resource tiers (no-alloc, alloc, std)

## Planned

### Fixture corpus

A `testdata/` corpus of real capability triples and known-bad inputs, providing a regression
suite and a confidence base for refactoring constraint logic.

### Fuzzing

Fuzz targets for the constraint engine and enumerator covering panic-safety, memory bounds,
and pipeline invariants under adversarial input.

### Broader constraint coverage

Additional built-in `ConstraintRule` implementations covering edge cases currently left to
callers: VRR range validation, Deep Color bandwidth margins, ALLM and QMS interaction checks.
