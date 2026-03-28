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

### `VideoMode::from_pixel_clock` for custom firmware timings (A1)

Firmware driving non-CTA modes constructs `VideoMode` via `VideoMode::new(w, h, refresh, interlace)`,
which derives the pixel clock via CVT-RB estimation. An exact-clock constructor in `display-types`
would let callers supply the pixel clock directly from their PLL or hardware register, making
bandwidth ceiling checks precise for non-standard modes. This is an upstream `display-types`
change; standard CTA modes already use `vic_to_mode` and are unaffected.

### VRR constraint implementation (I1)

`NegotiatedConfig.vrr_applicable` is always `false` today. Completing this requires
implementing VRR range validation (min/max refresh from the sink's VRR range descriptor) as a
`ConstraintRule` and updating `vrr_applicable` in `DefaultRanker` to reflect the result. Until
then the field's doc comment documents it as always-false.

### DSC parameter resolution (I2)

`DscCapabilities` captures `max_slices` and `max_bpp_x16` from the source, but neither is
validated in `DscCheck` nor surfaced in output. Completing this requires validating slice count
and BPP against source and sink limits and adding a resolved `DscConfig` struct (or equivalent
fields) to `NegotiatedConfig` so kernel drivers and firmware have actionable compression
parameters to program the encoder.
