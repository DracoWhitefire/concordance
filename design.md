# Concordance — Design Document

## Role

Given sink capabilities (from `piaf`) and source capabilities (a caller-supplied struct),
produce a ranked list of viable configurations. Answers: "what modes can I drive on this
display, in what priority order, using what color format and bit depth?"

Concordance is the policy layer of the stack. Parsing layers below it make no judgements.
Hardware layers above it implement specification. Concordance is explicitly opinionated, but
its opinions are configurable and its reasoning is always visible.

---

## Inputs and Output

**Inputs:**
- `&DisplayCapabilities` — from `piaf`; represents what the sink supports
- `SourceCapabilities` — a struct defined in this library, filled in by the caller
- `CableCapabilities` — a struct defined in this library, filled in by the caller

**Output:** A ranked iterator of `NegotiatedConfig`, each entry containing:
- Resolved `VideoMode`
- Color format and bit depth
- FRL tier (or TMDS, if applicable)
- DSC required flag
- VRR applicability
- `ReasoningTrace`

---

## SourceCapabilities

`SourceCapabilities` is a plain struct the caller fills in manually. Populating it from
actual GPU hardware is the concern of the source capability discovery library in the
integration layer, not this library.

```rust
#[non_exhaustive]
pub struct SourceCapabilities {
    pub max_tmds_clock: u32,
    pub frl_rates: BitFlags<FrlRate>,
    pub dsc: Option<DscCapabilities>,
    pub quirks: QuirkFlags,
    // ...
}
```

`#[non_exhaustive]` is used for forward compatibility. This struct represents real hardware
limits and may include vendor quirks.

---

## CableCapabilities

`CableCapabilities` is a plain struct the caller fills in manually. Populating it from
actual cable identification (e.g. HDMI cable type marker read from the sink EDID, or
user-supplied override) is the concern of the integration layer, not this library.

```rust
#[non_exhaustive]
pub struct CableCapabilities {
    pub hdmi_spec: HdmiSpec,        // e.g. Hdmi14, Hdmi20, Hdmi21
    pub max_frl_rate: Option<FrlRate>,  // None implies TMDS-only cable
    pub max_tmds_clock: u32,
    // ...
}
```

`HdmiSpec` encodes the cable's declared HDMI version, which determines the bandwidth
ceiling independently of what the source and sink can negotiate. A cable may be the
binding constraint even when both source and sink are HDMI 2.1 capable.

`CableCapabilities::unconstrained()` is provided as a convenience for callers that have
no cable information and wish to fall back to the optimistic assumption (source + sink
limits only).

---

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
    sink: &DisplayCapabilities,
    source: &SourceCapabilities,
    cable: &CableCapabilities,
    config: &CandidateConfig,
) -> Result<(), Vec<Violation>>
```

The ranked iterator is built on top of this primitive. Firmware and embedded consumers that
cannot afford allocation or iteration use this function directly.

### 2. Enumerator

Generates all possible candidate configurations from the intersection of sink, source, and
cable capabilities. Policy-free: it produces candidates, not preferences.

Custom enumerators can restrict or expand the candidate set (e.g. to limit enumeration to a
specific resolution list on embedded targets) without altering constraint or ranking logic.

### 3. Ranker

Orders the validated candidates according to a `NegotiationPolicy`. The default policy
encodes a sensible preference (native resolution, max color fidelity, then refresh rate,
then fallback formats), but the caller can supply an override.

Named policy presets (`BestQuality`, `BestPerformance`, `PowerSaving`) are a thin layer on
top of the same ranked iterator. Custom `NegotiationPolicy` implementations can encode
platform-specific priorities (e.g. always prefer a specific refresh rate, or penalize DSC).

---

## NegotiatedConfig and ReasoningTrace

Each entry in the ranked output includes a reasoning trace:

```rust
pub enum DecisionStep {
    Accepted { adjustments: Vec<Adjustment> },
    Rejected { reason: RejectionReason, details: String },
    PreferenceApplied { rule: PreferenceRule },
}

pub struct ReasoningTrace {
    pub steps: Vec<DecisionStep>,
}
```

Diagnostics are first-class and machine-readable. A compositor can ignore the trace; a
driver or diagnostic tool needs it.

---

## Consumer Perspectives

| Consumer             | What they want                                        |
|----------------------|-------------------------------------------------------|
| Compositor           | First valid entry from the ranked list; sane defaults |
| Driver / KMS bridge  | Full ranked list with reasoning trace; deterministic  |
| Firmware / embedded  | `is_config_viable` — no allocation, no iteration      |
| Test / validation    | Full enumeration including edge cases                 |
| End-user config tool | Named presets wrapping the ranked iterator            |

---

## Cable Consideration

HDMI link capability is determined by:

```
Source + Sink + Cable → Link Training → Actual Limits
```

Concordance takes an explicit `CableCapabilities` input and treats the cable as a first-class
constraint alongside source and sink. A cable that cannot carry FRL, or whose TMDS clock ceiling
is below the required pixel rate, produces a `Violation` like any other constraint failure.

Link training (in the SCDC/link training layer above) determines the real-world ceiling.
A `NegotiatedConfig` may still need to be revised downward after training, but the cable's
declared capabilities are enforced at negotiation time, not deferred.

Callers without cable information may pass `CableCapabilities::unconstrained()` to recover
the previous optimistic behavior.

---

## Design Principles

- **Ranked iterator, not a verdict.** There is no single right answer. The library
  enumerates all valid configurations in a defined, documented priority order and lets the
  caller pick. No mode is silently discarded — rejections appear in the trace.
- **No black box.** Every output entry carries enough context for a driver or diagnostic
  tool to explain the choice.
- **Configurable behavior.** Ranking priorities are governed by a `NegotiationPolicy` the
  caller supplies; named presets are provided for common cases. Constraint checking can be
  tuned via `QuirkFlags`. No behavioral choices are buried in the implementation.
- **Extensible without forking.** The three pipeline components (`ConstraintEngine`,
  `CandidateEnumerator`, `ConfigRanker`) are traits with default implementations. Any
  component can be replaced or wrapped via `NegotiatorBuilder` to accommodate
  platform-specific rules, restricted enumeration, or custom ranking, without touching the
  crate source.
- **`no_std` where it counts.** `is_config_viable` requires no allocation. The ranked
  iterator requires `alloc`. The crate is structured to allow `no_std + alloc` builds.
- **Stable output types.** `NegotiatedConfig` and `SourceCapabilities` are versioned output
  structs. Consumers are insulated from internal changes.

---

## Out of Scope

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
