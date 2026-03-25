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

### C2 — `TmdsClockCheck` emits the wrong violation variant

**File:** `src/engine/checks/timing.rs:153`, `src/output/warning.rs`
**Severity:** Medium

When the TMDS character rate exceeds the ceiling, `TmdsClockCheck` returns
`Violation::PixelClockExceeded`. The variant name, error message, and field names all say
"pixel clock" when the failing quantity is the TMDS character rate. Diagnostic tools and
humans reading violation output will misattribute the cause.

**Action:** Add `Violation::TmdsClockExceeded { required_mhz: u32, limit_mhz: u32 }` and use
it in `TmdsClockCheck`.

---

### C3 — YCbCr 4:2:0 capability is not per-mode

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

---

### C4 — `refresh_rate: u8` caps at 255 Hz

**File:** `display-types` (upstream), referenced in `src/engine/checks/timing.rs:24`
**Severity:** Low (today), rising

360 Hz panels are currently shipping. The `u8` field caps at 255. This is an upstream
`display-types` issue, but concordance inherits it and the HDMI 2.1 specification supports
rates above 255 Hz for lower resolutions.

**Action:** Track as an upstream issue against `display-types`. Note it in the roadmap.

---

## API ergonomics

### E1 — `SourceCapabilities` and `CableCapabilities` have no constructors

**File:** `src/types/source.rs`, `src/types/cable.rs`
**Severity:** Medium

Both types are `#[non_exhaustive]` with all-public fields. External crates cannot use struct
literal syntax (including the `..Default::default()` spread) — they must call
`Default::default()` and then assign fields individually. This is workable but asymmetric with
`SupportedModes::from_vec` and surprising to callers who try the natural struct literal form.

**Action:** Add named constructors (e.g. `SourceCapabilities::new(max_tmds_clock, max_frl_rate,
dsc, quirks)`) or a builder, so the construction path is obvious and compile-error-free.

---

### E2 — `CandidateConfig` construction in the README example

**File:** `README.md:31-37`
**Severity:** Medium

The README shows a struct literal for `CandidateConfig` in an external-crate context. If
`CandidateConfig` is `#[non_exhaustive]`, this example will not compile from outside the crate.
If it is not `#[non_exhaustive]`, adding a field is a breaking change.

**Action:** Verify the struct's `#[non_exhaustive]` status and reconcile with the example. If
it needs to remain constructible by literal (since it's a required input), document that
explicitly and ensure future fields have defaults.

---

### E3 — `QuirkFlags` is defined but empty

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

### O1 — Violations don't identify which party imposed the constraint

**File:** `src/output/warning.rs`, `src/engine/checks/timing.rs`
**Severity:** Medium

`PixelClockExceeded` gives `required_mhz` and `limit_mhz` but not whether the binding
ceiling came from the sink, source, or cable. `FrlRateExceeded` carries no context at all.
A compositor or diagnostic tool cannot reconstruct "this mode was rejected because the cable's
TMDS ceiling is too low" from the current output.

**Action:** Add a `source` field to bandwidth-related violations identifying which party
imposed the limit (`Sink`, `Source`, `Cable`, or `Tightest` for the combined minimum). This
is especially useful for cable-related rejections where the fix is "use a better cable."

---

### O2 — No rejection trace for non-accepted candidates

**File:** `src/builder.rs`, `src/output/`
**Severity:** Medium

`ReasoningTrace` is attached to `NegotiatedConfig` — accepted configs only. The pipeline
returns rejected candidates as a flat bag of `Violation`s with no per-candidate audit log.
A diagnostic tool that wants to show "why was 4K@120 HDR rejected?" must call
`is_config_viable` again and correlate manually.

**Action:** Add an opt-in rejection log to `NegotiatorBuilder` (behind `alloc`, e.g.
`.with_rejection_log()`) that collects `(CandidateConfig, Vec<Violation>)` pairs. This doesn't
need to be on by default — the allocation cost is non-trivial — but diagnostic consumers need
it.

---

### O3 — Rule names are not surfaced in violation output

**File:** `src/engine/rule.rs`, `src/output/warning.rs`
**Severity:** Low

`ConstraintRule::display_name()` returns a stable string identifier for each rule, but this
name does not appear in any `Violation` variant. Callers cannot tell which rule produced a
given violation without knowing the violation-to-rule mapping by convention.

**Action:** Consider adding a `rule: &'static str` field to violations, or a parallel
`(rule_name, Violation)` pair in the engine's output, so the rule name travels with its result.

---

## Incomplete features

### I1 — `vrr_applicable` is set but VRR validation is not implemented

**File:** `src/output/config.rs:38`
**Severity:** Medium

`NegotiatedConfig.vrr_applicable` is a public field on every output config. VRR constraint
checking is a roadmap item. If the field is always `false` today, callers who read it will get
incorrect results. If it is set to `true` optimistically, it overpromises.

**Action:** Document the field's current semantics explicitly in the doc comment ("always
`false` pending VRR constraint implementation") until the feature is complete. Alternatively,
gate it behind a `#[doc(hidden)]` or a `cfg` until it's real.

---

### I2 — `DscCheck` validates presence, not parameters

**File:** `src/engine/checks/dsc.rs`, `src/types/source.rs:17-28`, `src/output/config.rs`
**Severity:** Medium

`DscCapabilities` captures `max_slices` and `max_bpp_x16` from the source, but neither
appears in the constraint check or in `NegotiatedConfig` output. A kernel driver or firmware
enabling DSC needs a compression parameter set (slice count, BPP target) to program the
encoder. The current output acknowledges `dsc_required: true` but provides no actionable
parameters.

**Action:** Add resolved DSC parameters to `NegotiatedConfig` (or a nested `DscConfig` struct)
and validate slice count and BPP against source and sink limits in `DscCheck`. Track in the
roadmap if not addressing in 0.1.x.

---

## Audience-specific gaps

### A1 — No path from raw timing registers to `VideoMode` (firmware/embedded)

**File:** `README.md`, `doc/architecture.md`
**Severity:** Low

The embedded entry point (`is_config_viable`) requires a `CandidateConfig` with a `&VideoMode`.
Firmware that reads timing registers rather than EDID has no documented path to construct a
`VideoMode` from raw values (pixel clock, H/V active/blanking, sync polarity). The `VideoMode`
type in `display-types` may not have a public constructor for this.

**Action:** Document the intended construction path for firmware consumers who don't go through
EDID parsing, even if the answer is "populate `VideoMode::new(width, height, refresh, interlace)`
and accept estimate-based pixel clock checks."

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
