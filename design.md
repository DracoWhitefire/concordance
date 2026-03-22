# Concordance — Design Document

## Role

Given sink, source, and cable capabilities (all caller-supplied structs), produce a ranked
list of viable configurations. Answers: "what modes can I drive on this display, in what
priority order, using what color format and bit depth?"

Concordance is the policy layer of the stack. Parsing layers below it make no judgements.
Hardware layers above it implement specification. Concordance is explicitly opinionated, but
its opinions are configurable and its reasoning is always visible.

## Inputs and Output

**Inputs:**
- `SinkCapabilities` — a struct defined in this library, filled in by the caller
- `SourceCapabilities` — a struct defined in this library, filled in by the caller
- `CableCapabilities` — a struct defined in this library, filled in by the caller

**Output:** A ranked iterator of `NegotiatedConfig<W>`, each entry containing:
- Resolved `VideoMode`
- Color format and bit depth
- FRL tier (or TMDS, if applicable)
- DSC required flag
- VRR applicability
- `Vec<W>` — non-fatal warnings about the accepted configuration
- `ReasoningTrace`

## SinkCapabilities

`SinkCapabilities` is a plain struct the caller fills in manually. Populating it from a
parsed `DisplayCapabilities` (from `display-types`) is the concern of the integration layer,
not this library.

```rust
#[non_exhaustive]
pub struct SinkCapabilities {
    // Video modes declared by the display
    #[cfg(any(feature = "alloc", feature = "std"))]
    pub supported_modes: Vec<VideoMode>,

    // Timing range limits (from EDID range limits descriptor)
    pub max_pixel_clock_mhz: Option<u16>,
    pub min_v_rate: Option<u16>,
    pub max_v_rate: Option<u16>,

    // Color encoding (from EDID base block)
    pub digital_color_encoding: Option<DigitalColorEncoding>,
    pub color_bit_depth: Option<ColorBitDepth>,

    // HDMI 1.x capabilities (from HDMI VSDB; None if not present)
    pub hdmi_vsdb: Option<HdmiVsdb>,

    // HDMI 2.1 capabilities (from HF-SCDB; None for pre-HDMI-2.1 sinks)
    pub hdmi_forum: Option<HdmiForumSinkCap>,

    // HDR and colorimetry
    pub hdr_static: Option<HdrStaticMetadata>,
    pub colorimetry: Option<ColorimetryBlock>,
}
```

`VideoMode`, `DigitalColorEncoding`, `ColorBitDepth`, `HdmiVsdb`, `HdmiForumSinkCap`,
`HdrStaticMetadata`, and `ColorimetryBlock` are all from `display-types`. `supported_modes`
is absent in bare `no_std` builds; `is_config_viable` does not need the mode list since
it validates a caller-supplied candidate rather than enumerating one.

## SourceCapabilities

`SourceCapabilities` is a plain struct the caller fills in manually. Populating it from
actual GPU hardware is the concern of the source capability discovery library in the
integration layer, not this library.

```rust
#[non_exhaustive]
pub struct SourceCapabilities {
    pub max_tmds_clock: u32,
    pub max_frl_rate: HdmiForumFrl,
    pub dsc: Option<DscCapabilities>,
    pub quirks: QuirkFlags,
    // ...
}
```

`HdmiForumFrl` is from `display-types`. FRL rates are cumulative — declaring a maximum
implies support for all lower tiers — so a single `max_frl_rate` is the right
representation. `HdmiForumFrl::NotSupported` indicates a TMDS-only source.
`#[non_exhaustive]` is used for forward compatibility. This struct represents real hardware
limits and may include vendor quirks.

## CableCapabilities

`CableCapabilities` is a plain struct the caller fills in manually. Populating it from
actual cable identification (e.g. HDMI cable type marker read from the sink EDID, or
user-supplied override) is the concern of the integration layer, not this library.

```rust
#[non_exhaustive]
pub struct CableCapabilities {
    pub hdmi_spec: HdmiSpec,         // e.g. Hdmi14, Hdmi20, Hdmi21
    pub max_frl_rate: HdmiForumFrl,  // NotSupported = TMDS-only cable
    pub max_tmds_clock: u32,
    // ...
}
```

`HdmiSpec` is a concordance-defined enum encoding the cable's declared HDMI version.
`HdmiForumFrl` is from `display-types`. A cable may be the binding constraint even when
both source and sink are HDMI 2.1 capable.

`CableCapabilities::unconstrained()` is provided as a convenience for callers that have
no cable information and wish to fall back to the optimistic assumption (source + sink
limits only).

## Internal Architecture

The negotiation layer is structured into three components, each defined as a trait with a
default implementation. Callers can substitute any component without forking the crate.

```rust
pub trait ConstraintEngine { ... }
pub trait CandidateEnumerator { ... }
pub trait ConfigRanker { ... }
```

The components are wired together via `NegotiatorBuilder`, which accepts concrete
implementations for each slot and falls back to the defaults when none is supplied.

### 1. Constraint Engine

Determines whether a given configuration is valid for the supplied sink, source, and cable.
Returns structured violations, not just a boolean.

The default implementation enforces HDMI specification rules. Callers can wrap or replace it
to add vendor-specific constraint rules (e.g. platform bandwidth caps, quirk overrides)
without touching the rest of the pipeline.

This is also exposed directly as the `no_std`-compatible binary probe:

```rust
pub fn is_config_viable(
    sink: &SinkCapabilities,
    source: &SourceCapabilities,
    cable: &CableCapabilities,
    config: &CandidateConfig,
) -> Result<(), Vec<Violation>>
```

The ranked iterator is built on top of this primitive. Firmware and embedded consumers that
cannot afford allocation or iteration use this function directly.

### 2. Enumerator

Generates all candidate configurations from the intersection of sink, source, and cable
capabilities. Completely policy-free: the enumerator produces candidates; it never
pre-filters based on perceived usefulness. No candidate is dropped at enumeration time —
rejection happens only in the constraint engine.

Equivalent candidates (same mode, format, and tier reached by different paths) are
deduplicated by the pipeline before ranking.

Custom enumerators can restrict or expand the candidate set (e.g. to limit enumeration to a
specific resolution list on embedded targets) without altering constraint or ranking logic.

### 3. Ranker

Orders the validated candidates according to a `NegotiationPolicy`. The default policy
encodes a sensible preference (native resolution, max color fidelity, then refresh rate,
then fallback formats), but the caller can supply an override.

Named policy presets (`BestQuality`, `BestPerformance`, `PowerSaving`) are a thin layer on
top of the same ranked iterator. Custom `NegotiationPolicy` implementations can encode
platform-specific priorities (e.g. always prefer a specific refresh rate, or penalize DSC).

## NegotiatedConfig and ReasoningTrace

`NegotiatedConfig` is a pure data struct — it holds resolved values. Helpers that compute
derived results (compatibility checks, ranking utilities, mode filters) are free functions in
separate modules, not methods on the struct. This keeps the output type stable even as
higher-level policy evolves.

`NegotiatedConfig` is generic over the warning type, defaulting to the built-in `Warning`:

```rust
pub struct NegotiatedConfig<W = Warning> {
    // resolved fields ...
    pub warnings: Vec<W>,
    pub trace: ReasoningTrace,
}

/// Bound required on all warning types, built-in or custom.
pub trait Diagnostic: fmt::Display + fmt::Debug {}

#[derive(Debug, thiserror::Error)]
pub enum Warning {
    #[error("DSC required; lossy compression active")]
    DscActive,
    #[error("cable bandwidth marginal for selected mode")]
    CableBandwidthMarginal,
    // ...
}
impl Diagnostic for Warning {}

pub enum DecisionStep {
    Accepted { adjustments: Vec<Adjustment> },
    Rejected { reason: RejectionReason, details: String },
    PreferenceApplied { rule: PreferenceRule },
}

pub struct ReasoningTrace {
    pub steps: Vec<DecisionStep>,
}
```

`ConstraintEngine` and `ConfigRanker` each declare an associated `Warning` type bounded by
`Diagnostic`, so a custom component can emit its own warning variants without wrapping or
losing type information. The default implementations use the built-in `Warning`.

Warnings are attached to accepted configurations — they do not prevent a mode from being
offered, but give the caller enough information to surface concerns to the user or log them.
Diagnostics are first-class and machine-readable. A compositor can ignore both; a driver or
diagnostic tool needs them.

## Error Handling

Fatal errors (invalid inputs, internal invariant violations) are represented as a
`thiserror`-derived `Error` type returned from fallible API entry points.

`Violation`, used in `is_config_viable`, is a `thiserror` error type with an associated
type on `ConstraintEngine`:

```rust
pub trait ConstraintEngine {
    type Warning: Diagnostic;
    type Violation: Diagnostic;
    // ...
}
```

A custom `ConstraintEngine` implementation can define its own `Violation` type — adding
platform-specific rejection reasons — without wrapping the built-in enum or losing
structured information. The built-in `Violation` type remains the default.

Real hardware often declares inconsistent or conflicting capabilities. Where possible,
concordance produces the best available output and surfaces the inconsistency as a warning
rather than refusing to negotiate. Callers decide how strict to be.

`thiserror` is a build-time dependency only; it generates no runtime overhead.

## Consumer Perspectives

| Consumer             | What they want                                        |
|----------------------|-------------------------------------------------------|
| Compositor           | First valid entry from the ranked list; sane defaults |
| Driver / KMS bridge  | Full ranked list with reasoning trace; deterministic  |
| Firmware / embedded  | `is_config_viable` — no allocation, no iteration      |
| Test / validation    | Full enumeration including edge cases                 |
| End-user config tool | Named presets wrapping the ranked iterator            |

## Cable Consideration

HDMI link capability is determined by:

```
Source + Sink + Cable → Link Training → Actual Limits
```

Concordance takes an explicit `CableCapabilities` input and treats the cable as a first-class
constraint alongside source and sink. A cable that cannot carry FRL, or whose TMDS clock
ceiling is below the required pixel rate, produces a `Violation` like any other constraint
failure.

Link training (in the SCDC/link training layer above) determines the real-world ceiling.
A `NegotiatedConfig` may still need to be revised downward after training, but the cable's
declared capabilities are enforced at negotiation time, not deferred.

Callers without cable information may pass `CableCapabilities::unconstrained()` to recover
the previous optimistic behavior.

## Design Principles

- **Ranked iterator, not a verdict** — there is no single right answer. The library
  enumerates all valid configurations in a defined, documented priority order and lets the
  caller pick. No mode is silently discarded — rejections appear in the trace.
- **No black box** — every output entry carries enough context for a driver or diagnostic
  tool to explain the choice.
- **Configurable behavior** — ranking priorities are governed by a `NegotiationPolicy` the
  caller supplies; named presets are provided for common cases. Constraint checking can be
  tuned via `QuirkFlags`. No behavioral choices are buried in the implementation.
- **Extensible without forking** — the three pipeline components (`ConstraintEngine`,
  `CandidateEnumerator`, `ConfigRanker`) are traits with default implementations. Any
  component can be replaced or wrapped via `NegotiatorBuilder` to accommodate
  platform-specific rules, restricted enumeration, or custom ranking, without touching the
  crate source. Warning and violation types are associated types on these traits, so custom
  components can emit their own diagnostic variants with full type fidelity.
- **`NegotiatedConfig` is a data struct, not a decision layer** — it holds resolved values.
  Derived operations live as free functions in separate modules, not as methods on the struct.
- **Tiered resource model** — three audiences are explicitly supported, each with its own
  build profile:
  - `no_std`, no alloc, no copy — `is_config_viable` borrows all inputs and returns
    structured violations with no heap use. Targets firmware and embedded consumers.
  - `no_std + alloc` — the ranked iterator and `ReasoningTrace` require allocation but
    avoid unnecessary copies; inputs are still borrowed throughout.
  - `std` — full feature set; additive on top of `alloc`.
  Borrowing is the default throughout the API. Owned types appear only where the output
  genuinely needs to outlive its inputs.
- **Stable output types** — `NegotiatedConfig` and the three input structs are
  `#[non_exhaustive]` and versioned. Consumers are insulated from internal changes.
- **No unsafe code** — `#![forbid(unsafe_code)]` is a hard constraint, not a guideline.
- **Serde on all public types** — every public type derives `Serialize`/`Deserialize` behind
  a `serde` feature flag, covering inputs, outputs, and policy types. Enables diagnostic
  tooling, config persistence, and test fixtures without making serde a required dependency.

## Testing

Negotiation logic benefits from a testing approach that combines small deterministic tests
with larger corpus-based validation.

### Unit tests

Unit tests cover narrow pieces of logic and live next to the code they test. Constraint
engine tests call `check` directly on handcrafted capability structs without going through
the full pipeline. Enumerator tests assert on the candidate set produced from a given
capability triple. This keeps failures localized: a failing test in the engine can only
mean the engine is broken.

### Integration tests

A single integration test verifies that `NegotiatorBuilder::default()` wires the pipeline
correctly and that `negotiate` invokes all three components. It does not duplicate the
field-level assertions that belong in component unit tests.

### Fixture tests

Concordance should maintain a fixture corpus containing:

- valid capability triples from real hardware,
- capability declarations with known inconsistencies,
- edge cases (TMDS-only cable, DSC required, VRR boundary conditions),
- pathological inputs.

A suggested layout:
```text
testdata/
 ├── valid/
 ├── invalid/
 └── edge/
```

Fixtures serve as a regression suite and a confidence base for refactoring negotiation logic
without unintentionally changing behaviour.

### Fuzzing

Fuzzing is strongly recommended for the constraint engine and enumerator.

Important expectations:

- no panics,
- no uncontrolled memory growth,
- any input produces controlled output (violations, warnings) rather than undefined behaviour,
- unknown or conflicting capability values do not break pipeline invariants.

## Out of Scope

- **Sink capability discovery** — parsing EDID and HF-VSDB into `SinkCapabilities` belongs
  in the parsing layer (e.g. `piaf`). The integration layer converts parsed output into this
  library's `SinkCapabilities` struct. Concordance consumes it; it does not produce it.
- **Source capability discovery** — querying DRM/KMS or VBIOS for actual GPU limits belongs
  in the integration layer. Concordance consumes `SourceCapabilities`; it does not produce it.
- **Cable capability discovery** — reading the HDMI cable type marker from the sink EDID or
  accepting a user-supplied override belongs in the integration layer. Concordance consumes
  `CableCapabilities`; it does not produce it.
- **Link training** — determining whether a negotiated FRL tier is achievable on real
  hardware is the concern of the SCDC/link training layer.
- **InfoFrame encoding** — signaling the negotiated configuration to the sink is handled by
  the InfoFrame library.
- **HDCP** — out of scope for the entire stack.
