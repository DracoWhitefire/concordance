# Enumerator Design

The enumerator generates every `CandidateConfig` that is worth evaluating — the full
Cartesian product of modes, color encodings, bit depths, FRL tiers, and DSC state — and
hands them to the constraint engine one at a time. It is the source of candidates; it is
not a filter.

## Role in the pipeline

```
SinkCapabilities
SourceCapabilities   ──►  CandidateEnumerator  ──►  (mode, encoding, depth, frl, dsc)
CableCapabilities                                            │
                                                            ▼
                                                   ConstraintEngine
                                                            │
                                                            ▼
                                                       ConfigRanker
```

The enumerator is called once per negotiation run and returns a lazy iterator. It does
not own results, allocate per-candidate, or inspect violations — those are strictly the
engine's concern.

## The trait

```rust
pub trait CandidateEnumerator {
    type Iter<'a>: Iterator<Item = CandidateConfig<'a>>
    where
        Self: 'a;

    fn enumerate<'a>(
        &'a self,
        sink:   &'a SinkCapabilities,
        source: &'a SourceCapabilities,
        cable:  &'a CableCapabilities,
    ) -> Self::Iter<'a>;
}
```

`Iter` is a generic associated type (GAT), so each implementor can return its own
concrete iterator without boxing. Both built-in enumerators return `EnumeratorIter<'a>`.

The trait is the plug-in point for `NegotiatorBuilder`:

```rust
NegotiatorBuilder::default()                    // En = DefaultEnumerator
    .with_enumerator(SliceEnumerator::new(&modes)) // En = SliceEnumerator<'_>
```

`NegotiatorBuilder` is generic over `En: CandidateEnumerator`; the enumerator slot is
statically dispatched — no `dyn`, no allocation.

`enumerate` is called once per `NegotiatorBuilder::negotiate` call. The returned
iterator is driven to exhaustion by the pipeline; the enumerator itself is not mutated.

## The two concrete types

### `SliceEnumerator<'modes>`

Available in all feature tiers (bare `no_std`, `alloc`, `std`).

The caller provides the mode list as a borrowed slice:

```rust
let enumerator = SliceEnumerator::new(&sink.supported_modes);
```

This is the right choice for embedded targets and for tests that want a controlled mode set.

### `DefaultEnumerator`

Available in `alloc` and `std` tiers only (because `SinkCapabilities::supported_modes`
requires `Vec`).

It borrows `sink.supported_modes` inside `enumerate()` — no separate construction
argument is needed:

```rust
let enumerator = DefaultEnumerator;
// …used via NegotiatorBuilder::default()
```

Both types return the same `EnumeratorIter<'a>` and differ only in where the mode slice
comes from.

## Candidate dimensions

The Cartesian product spans five dimensions, in this order (outermost first):

| # | Dimension      | Values                                              |
|---|----------------|-----------------------------------------------------|
| 1 | `mode`         | every `VideoMode` in the provided slice             |
| 2 | `color_encoding` | subset of `[Rgb444, YCbCr444, YCbCr422, YCbCr420]` |
| 3 | `bit_depth`    | subset of `[Depth8, Depth10, Depth12, Depth16]`     |
| 4 | `frl_rate`     | subset of the seven `HdmiForumFrl` tiers            |
| 5 | `dsc_enabled`  | `[false]` or `[false, true]`                        |

The total candidate count is `|modes| × |encodings| × |depths| × |frl_rates| × |dsc|`.
For a typical sink (40 modes, 2 encodings, 2 depths, 4 FRL tiers, no DSC) this is
around 640 candidates — well within the constraint engine's per-candidate cost.

## Pre-filtering at construction time

The enumerator computes its dimension arrays once when `enumerate()` is called, not
per-candidate. Three axes are pre-filtered against the capability triple:

**Color encodings** — include only those where the sink declares at least one bit depth:

```rust
sink.color_capabilities.for_format(enc).is_nonempty()
```

**Bit depths** — include the union of all depths supported by the sink across any
encoding. A candidate combining an encoding with an unsupported depth will be cheaply
rejected by `BitDepthCheck`; the minor over-enumeration avoids per-encoding depth arrays
in the iterator state.

**FRL rates** — include only tiers ≤ `min(source_ceil, sink_ceil, cable_ceil)`, plus
`NotSupported` (TMDS) always. This avoids generating large numbers of candidates that
`FrlCeilingCheck` would immediately reject.

**DSC** — include `true` only when both source and sink support DSC (`dsc_1p2`). When
DSC is unavailable, the `dsc_enabled = false` dimension collapses to a single element.

This pre-filtering is a performance optimisation, not a policy decision. The constraint
engine remains authoritative: every candidate the enumerator emits is subject to the full
check list. Pre-filtering cannot cause a viable candidate to be silently skipped.

## Iterator: `EnumeratorIter<'a>`

A lazy struct that advances through the Cartesian product without any heap allocation.
All dimension arrays are fixed-size and stored inline.

```rust
pub struct EnumeratorIter<'a> {
    modes:    &'a [VideoMode],
    encodings: [ColorFormat; 4],  enc_len: usize,
    depths:    [ColorBitDepth; 4], dep_len: usize,
    frl_rates: [HdmiForumFrl; 7], frl_len: usize,
    dsc:       [bool; 2],          dsc_len: usize,

    // current position (odometer, rightmost index is innermost)
    mode_idx: usize,
    enc_idx:  usize,
    dep_idx:  usize,
    frl_idx:  usize,
    dsc_idx:  usize,
}
```

`Iterator::next()` advances the innermost index first, carrying into the next index when
it wraps, exactly like an odometer. When `mode_idx == modes.len()` the iterator is
exhausted.

The candidate borrows `&self.modes[mode_idx]` directly — no copy until acceptance in
`NegotiatedConfig`.

## Iteration order

Candidates are produced in the following order (first dimension changes slowest):

```
mode[0] × encoding[0] × depth[0] × frl[0] × dsc[0]
mode[0] × encoding[0] × depth[0] × frl[0] × dsc[1]
mode[0] × encoding[0] × depth[0] × frl[1] × dsc[0]
…
mode[0] × encoding[0] × depth[1] × …
…
mode[1] × …
```

This order ensures all candidates for a given mode are emitted consecutively, which is
cache-friendly and easy to reason about in traces.

## Mode list deduplication

The enumerator does not deduplicate its input. Responsibility lies with the caller that
assembles `SinkCapabilities`.

Duplicate `VideoMode` entries are a real occurrence: piaf can emit the same timing from
multiple sources in a single EDID — an established timing, a standard timing, and a CEA
Video Data Block VIC can all resolve to the same `(width, height, refresh_rate)` tuple.
`DisplayCapabilities::supported_modes` preserves all of them, and `sink_capabilities_from_display`
clones that list verbatim.

The correct fix is in `sink_capabilities_from_display`: sort `supported_modes` and
deduplicate before storing, so every downstream consumer — the enumerator included —
receives a normalised list. This is cheap (the `Vec` is already owned) and makes the
invariant unconditional rather than something each consumer has to defend against.

A custom enumerator that constructs its own mode list is responsible for the same
invariant.

## What is out of scope

- **Ranking** — no preference or ordering beyond the iteration order defined above.
- **Preferred-mode pinning** — the preferred mode (`VideoMode::preferred`) is not treated
  specially; the ranker assigns higher weight to it.
- **Y420 per-mode VIC filtering** — the CEA-861 `y420_capability_map` declares which
  specific VICs support YCbCr 4:2:0. The enumerator does not consult it; `ColorEncodingCheck`
  is the appropriate place for that check once per-mode metadata is plumbed through.
