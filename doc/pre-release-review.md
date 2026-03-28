# Pre-release review

Critical review of concordance 0.1.0 from a potential implementer's perspective, covering
hardware/firmware, kernel/driver, diagnostic tooling, and compositor audiences.

Items are grouped by type. Each has a short action note.

---

## Correctness

### C1 — Pixel clock function name obscured its exact-or-estimate behavior ✓ resolved

**File:** `display-types/src/timing.rs`, `src/engine/checks/timing.rs`
**Severity:** Medium (naming); Low (correctness)

`pixel_clock_khz_cvt_rb_estimate` returned the exact DTD pixel clock when present and only
fell back to CVT-RB estimation for modes without a Detailed Timing Descriptor. The name
implied estimation always occurred, misleading callers about accuracy.

The CVT-RB fallback itself under-estimates for HDMI Forum-specified CTA modes (e.g. 4K@60,
VIC 97) by ~10–15%, which can produce false accepts in bandwidth ceiling checks. This is a
known limitation documented in the function's doc comment; a VIC lookup table would be needed
to fully address it.

**Resolution:** Renamed to `pixel_clock_khz` in `display-types`. Doc comment updated to
accurately describe the exact-or-estimate logic and call out the under-estimation risk for
CTA modes without DTDs.

---

### C2 — `TmdsClockCheck` emits the wrong violation variant ✓ resolved

**File:** `src/engine/checks/timing.rs`, `src/output/warning.rs`
**Severity:** Medium

When the TMDS character rate exceeded the ceiling, `TmdsClockCheck` returned
`Violation::PixelClockExceeded`. The variant name, error message, and field names all said
"pixel clock" when the failing quantity is the TMDS character rate.

**Resolution:** Added `Violation::TmdsClockExceeded { required_mhz: u32, limit_mhz: u32 }`
and updated `TmdsClockCheck` to emit it. All affected tests updated.

---

### C3 — YCbCr 4:2:0 capability is not per-mode ✓ resolved

**File:** `src/types/sink.rs:140-143`
**Severity:** Medium

`sink_capabilities_from_display` sets the global `ycbcr420` capability when any Y420 Video
Data Block or Y420 Capability Map Data Block is present. But the Y420 VDB lists specific VICs
— only those modes support 4:2:0. The constraint engine will permit 4:2:0 on modes where the
sink actually rejects it.

**Action:** Either store per-mode encoding constraints (a map from `VideoMode` / VIC to allowed
encodings), or document the current behavior as a conservative approximation and track the
limitation. Until fixed, the color encoding check may produce false accepts for 4:2:0 on
non-listed modes.

What the EDID data actually says

There are three independent sources for YCbCr 4:2:0 capability, each with different semantics:

Y420 Video Data Block (y420_vics: Vec<u8>) — these VICs support only 4:2:0. Other encodings are invalid for them. The current code ignores this entirely; the enumerator will happily emit RGB        
candidates for these modes.

Y420 Capability Map Data Block (y420_capability_map: Vec<u8>) — a bitmap over the ordered SVD list (vics: Vec<(u8, bool)>). These modes support 4:2:0 in addition to their declared encodings.        
Resolving this is mechanical: bit N of the bitmap → vics[N] → VIC number.

HF-SCDB deep color flags (dc_30bit_420, dc_36bit_420, dc_48bit_420) — display-level; declare which bit depths are supported for 4:2:0 where it applies. Not a mode list.

The current design only has color_capabilities.ycbcr420: ColorBitDepths, which flattens all three into a per-display bit depth set. This loses the mode dimension entirely and conflates deep color   
depth capability with mode eligibility.
                                                                                                                                                                                                        
---                                                       
Proposed model

Two new fields on SinkCapabilities:

/// Modes that support *only* YCbCr 4:2:0 (from the Y420 Video Data Block).
/// Other color encodings must be rejected for these modes.                                                                                                                                           
pub ycbcr420_exclusive_modes: SupportedModes,

/// Modes that *also* support YCbCr 4:2:0 (from the Y420 Capability Map Data Block).                                                                                                                  
/// Other encodings remain valid; 4:2:0 is an additional option.
pub ycbcr420_capable_modes: SupportedModes,

color_capabilities.ycbcr420 stays but its role narrows to bit depth capability (from HF-SCDB), used once mode eligibility is established.

ColorEncodingCheck gains three cases for YCbCr420:
1. Candidate mode is in ycbcr420_exclusive_modes or ycbcr420_capable_modes → allowed
2. Neither list is populated and color_capabilities.ycbcr420 is non-empty → allowed (fallback, current behavior, with a warning)
3. Otherwise → ColorEncodingUnsupported

It also needs a new check (or integration into ColorEncodingCheck): if the candidate mode is in ycbcr420_exclusive_modes and the encoding is not YCbCr420 → rejection, new                            
Violation::EncodingRestrictedToYCbCr420.
                                                                                                                                                                                                        
---                                                                                                                                                                                                   
The VIC resolution question

sink_capabilities_from_display needs to turn VIC numbers into VideoMode structs. For the CMB this is y420_capability_map bitmap × vics → VIC numbers. For the VDB it's y420_vics directly. Both end at
VIC numbers.

The comment on vics says "VICs beyond the range of the built-in lookup table are included here but do not produce an entry
in DisplayCapabilities::supported_modes".

Resolution is straightforward. The VIC table is in display-types, and it is public: display_types::cea861::vic_table pub fn vic_to_mode
src/types/sink.rs — add two new SupportedModes fields; update sink_capabilities_from_display to populate them from VDB (direct VIC lookup) and CMB (bitmap × vics → VIC lookup); keep the existing    
BPC_8 addition to color_capabilities.ycbcr420 (it serves BitDepthCheck).

src/output/warning.rs — add Violation::EncodingRestrictedToYCbCr420 for non-4:2:0 encodings on exclusive modes.

src/engine/checks/color.rs — update ColorEncodingCheck with a #[cfg(any(feature = "alloc", feature = "std"))] block that handles exclusive mode rejection and per-mode 4:2:0 eligibility with a       
fallback for manually-constructed SinkCapabilities where both lists are empty.

The no-alloc path is untouched — is_config_viable callers supply their own CandidateConfig and the check falls back to the existing display-level color_capabilities.ycbcr420 test.



---

### C4 — `refresh_rate: u8` caps at 255 Hz ✓ resolved

**File:** `display-types` (upstream), referenced in `src/engine/checks/timing.rs:24`
**Severity:** Low (today), rising

360 Hz panels are currently shipping. The `u8` field caps at 255. This is an upstream
`display-types` issue, but concordance inherits it and the HDMI 2.1 specification supports
rates above 255 Hz for lower resolutions.

**Action:** Track as an upstream issue against `display-types`. Note it in the roadmap.

---

## API ergonomics

### E1 — `SourceCapabilities` and `CableCapabilities` have no constructors ✓ resolved

**File:** `src/types/source.rs`, `src/types/cable.rs`
**Severity:** Medium

Both types are `#[non_exhaustive]` with all-public fields. External crates cannot use struct
literal syntax (including the `..Default::default()` spread) — they must call
`Default::default()` and then assign fields individually. This is workable but asymmetric with
`SupportedModes::from_vec` and surprising to callers who try the natural struct literal form.

**Resolution:** Added `SourceCapabilities::new(max_tmds_clock, max_frl_rate, dsc)` and
`CableCapabilities::new(hdmi_spec, max_frl_rate, max_tmds_clock)` as `const fn` constructors.
Quirks default to `QuirkFlags::empty()` and can be set via field assignment after construction.
Signatures are intentionally minimal for now and will be revisited when the field set stabilises.

---

### E2 — `CandidateConfig` construction in the README example ✓ resolved

**File:** `README.md:31-37`
**Severity:** Medium

The README shows a struct literal for `CandidateConfig` in an external-crate context. If
`CandidateConfig` is `#[non_exhaustive]`, this example will not compile from outside the crate.
If it is not `#[non_exhaustive]`, adding a field is a breaking change.

**Action:** Verify the struct's `#[non_exhaustive]` status and reconcile with the example. If
it needs to remain constructible by literal (since it's a required input), document that
explicitly and ensure future fields have defaults.

---

### E3 — `QuirkFlags` is defined but empty ✓ resolved

**File:** `src/types/source.rs:12-15`
**Severity:** Low

`QuirkFlags` is a `bitflags` type with no defined bits, documented as "reserved for
platform-specific flags." External crates cannot add entries to a foreign `bitflags` type, so
the field is currently inert and its future design is unclear.

**Action:** Either define the first real quirk flag (even a placeholder with documentation) to
show the intended pattern, or replace the field with a `u32` with documented bit semantics that
callers can extend, or document that quirks are a future internal mechanism only.

---

### E4 — `POWER_SAVING` policy has `prefer_native_resolution: true`

**File:** `src/ranker/policy.rs:61-66`
**Severity:** Low

The `POWER_SAVING` preset minimizes bandwidth but also prefers native resolution. Non-native
resolutions typically consume less bandwidth. The combination may be intentional ("minimize
bandwidth within native res only") but it's not explained and reads as an oversight.

**Action:** Add a comment on the preset explaining the intent, or reconsider the flag value.

---

## Observability gaps

### O1 — Violations don't identify which party imposed the constraint ✓ resolved

**File:** `src/output/warning.rs`, `src/engine/checks/timing.rs`, `src/engine/checks/frl.rs`
**Severity:** Medium

`PixelClockExceeded` gives `required_mhz` and `limit_mhz` but not whether the binding
ceiling came from the sink, source, or cable. `FrlRateExceeded` carries no context at all.
A compositor or diagnostic tool cannot reconstruct "this mode was rejected because the cable's
TMDS ceiling is too low" from the current output.

**Resolution:** Added `pub enum LimitSource { Sink, Source, Cable }` (with `Display`) to
`warning.rs` and re-exported from the crate root. Added `limit_source: LimitSource` to
`PixelClockExceeded` and `TmdsClockExceeded`. Expanded `FrlRateExceeded` from a unit variant
to a struct with `requested: HdmiForumFrl`, `limit: HdmiForumFrl`, and
`limit_source: LimitSource`. When multiple parties share the binding tier or clock value,
`Cable` takes priority over `Source` over `Sink` — cable replacement is the most actionable
fix for the end user. `PixelClockExceeded` always reports `LimitSource::Sink` because the
pixel clock ceiling is sourced exclusively from the EDID range limits descriptor.

---

### O2 — No rejection trace for non-accepted candidates ✓ resolved

**File:** `src/builder.rs`, `src/output/rejection.rs`
**Severity:** Medium

`ReasoningTrace` is attached to `NegotiatedConfig` — accepted configs only. The pipeline
returns rejected candidates as a flat bag of `Violation`s with no per-candidate audit log.
A diagnostic tool that wants to show "why was 4K@120 HDR rejected?" must call
`is_config_viable` again and correlate manually.

**Resolution:** Added `RejectedConfig<V>` to `src/output/rejection.rs` — an owned mirror of
`CandidateConfig` plus `violations: Vec<V>`. Added
`NegotiatorBuilder::negotiate_with_log()` returning
`(Vec<NegotiatedConfig<W>>, Vec<RejectedConfig<V>>)`. The existing `negotiate()` is
unchanged and allocates no rejection log. Both methods share a private `negotiate_inner`
so there is no logic duplication. `RejectedConfig` is re-exported from the crate root.

---

### O3 — Rule names are not surfaced in violation output ✓ resolved

**File:** `src/engine/rule.rs`, `src/output/warning.rs`, `src/engine/mod.rs`, `src/builder.rs`
**Severity:** Low

`ConstraintRule::display_name()` returns a stable string identifier for each rule, but this
name does not appear in any `Violation` variant. Callers cannot tell which rule produced a
given violation without knowing the violation-to-rule mapping by convention.

**Resolution:** Added `pub struct TaggedViolation<V = Violation> { rule: &'static str, violation: V }`
to `output/warning.rs`. `DefaultConstraintEngine` now has `type Violation = TaggedViolation<V>` and
tags each violation at collection time with `rule.display_name()`. `TaggedViolation<V>` implements
`Display` as `"[rule_name] message"` and is re-exported from the crate root.

Added `TaggingAdapter<R>` to `engine/rule.rs` — wraps a `ConstraintRule<V>` into a
`ConstraintRule<TaggedViolation<V>>` so custom rules can be composed with the layered engine.
`NegotiatorBuilder::with_extra_rule` now accepts a `ConstraintRule<V>` (inner type) and wraps
it in `TaggingAdapter` automatically — callers implement rules against the inner violation type
and receive tagged output at the engine boundary without any extra ceremony.

`is_config_viable` return type updated to `CheckResult<Warning, TaggedViolation<Violation>>`.
`TaggedViolation` is `#[non_exhaustive]` and derives serde `Serialize`; deserialization sets
`rule` to `""` (round-tripping `&'static str` from runtime data requires leaking).

---

## Incomplete features

### I1 — `vrr_applicable` is set but VRR validation is not implemented → deferred to roadmap

**File:** `src/output/config.rs:38`
**Severity:** Medium

`NegotiatedConfig.vrr_applicable` is a public field on every output config. VRR constraint
checking is a roadmap item. If the field is always `false` today, callers who read it will get
incorrect results. If it is set to `true` optimistically, it overpromises.

**Resolution:** Field doc comment updated to document the current always-`false` semantics
explicitly ("always `false` pending VRR constraint implementation"). Full VRR range validation
is tracked in `doc/roadmap.md` under "VRR constraint implementation".

---

### I2 — `DscCheck` validates presence, not parameters → deferred to roadmap

**File:** `src/engine/checks/dsc.rs`, `src/types/source.rs:17-28`, `src/output/config.rs`
**Severity:** Medium

`DscCapabilities` captures `max_slices` and `max_bpp_x16` from the source, but neither
appears in the constraint check or in `NegotiatedConfig` output. A kernel driver or firmware
enabling DSC needs a compression parameter set (slice count, BPP target) to program the
encoder. The current output acknowledges `dsc_required: true` but provides no actionable
parameters.

**Resolution:** Deferred to post-0.1.x. Tracked in `doc/roadmap.md` under "DSC parameter
resolution". The `dsc_required` field in `NegotiatedConfig` retains its current semantics
(boolean presence check) until the full implementation lands.

---

## Audience-specific gaps

### A1 — No path from raw timing registers to `VideoMode` (firmware/embedded) ✓ partially resolved

**File:** `README.md`, `doc/architecture.md`
**Severity:** Low

The embedded entry point (`is_config_viable`) requires a `CandidateConfig` with a `&VideoMode`.
Firmware that reads timing registers rather than EDID has no documented path to construct a
`VideoMode` from raw values (pixel clock, H/V active/blanking, sync polarity). The `VideoMode`
type in `display-types` may not have a public constructor for this.

**Resolution:** Two paths documented in `README.md` and `doc/architecture.md`:

- **Standard CTA modes** — `display_types::cea861::vic_to_mode(vic)` returns a `VideoMode`
  with the exact pixel clock from the CEA-861 timing table. README example updated to show
  this path. No API changes required.
- **Custom / non-CTA timings** — `VideoMode::new(width, height, refresh_hz, interlace)` with
  the caveat that pixel clock is CVT-RB estimated (may under-estimate ~10–15% for HDMI Forum
  CTA modes). A `VideoMode::from_pixel_clock` constructor for exact-clock custom timings is
  planned as an upstream `display-types` feature and tracked in `doc/roadmap.md`.

---

### A2 — `NegotiatedConfig` does not carry a caller-supplied token (kernel/driver)

**File:** `src/output/config.rs`
**Severity:** Low

After negotiation, a DRM driver needs to find the KMS mode blob matching the `NegotiatedConfig`.
`VideoMode` carries width/height/refresh/interlace, which is not always sufficient to uniquely
identify a KMS mode (two detailed timing descriptors at the same resolution and refresh rate but
different pixel clocks are common). There is no way to attach a caller-supplied token (e.g. a
KMS mode ID) to a `VideoMode` so it survives the pipeline round-trip.

**Action:** Consider a `CandidateConfig.tag: Option<u64>` or a generic tag parameter passed
through to `NegotiatedConfig`. Alternatively, document the KMS correlation strategy explicitly.

---

### A3 — `NegotiationPolicy` has no preferred refresh or resolution floor (compositor)

**File:** `src/ranker/policy.rs`
**Severity:** Low

The three presets rank all valid configs globally. A Wayland compositor managing multi-monitor
output needs to express "prefer 60 Hz on this output" or "exclude anything below 1080p" as
policy inputs, without replacing the full ranker. Currently this requires a custom `ConfigRanker`
implementation.

**Action:** Add optional `preferred_refresh_hz` and `min_resolution_pixels` fields to
`NegotiationPolicy`, or document the recommended pattern for compositors that need soft
preferences (e.g., post-rank filter + `with_extra_rule` for hard filters).

---

## Documentation

### D1 — `doc/architecture.md` is not linked from `docs.rs`

**File:** `src/lib.rs`
**Severity:** Low

`doc/architecture.md` contains the most complete description of the pipeline, design
principles, and constraint rules. Readers arriving at `docs.rs` have no path to it.

**Action:** Add a module-level doc comment in `src/lib.rs` with a prose overview and a
reference to the extended documentation, or use `#![doc = include_str!("../doc/architecture.md")]`
(or a subset) to surface it directly.

---

### D2 — `CableCapabilities::unconstrained()` name may encourage permanent use

**File:** `src/types/cable.rs`
**Severity:** Low

The escape hatch for callers without cable information is clearly useful, but the name
`unconstrained()` doesn't communicate that this disables real constraints. Callers under
time pressure may leave it permanently.

**Action:** Rename to `CableCapabilities::unknown()` or add a doc comment warning that this
bypasses cable-related violations and should be replaced once cable detection is available.
