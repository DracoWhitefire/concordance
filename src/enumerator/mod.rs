//! Candidate enumerator trait and implementations.

use display_types::cea861::HdmiForumFrl;
use display_types::{ColorBitDepth, ColorFormat, VideoMode};

use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

/// Generates all candidate configurations from the intersection of capabilities.
///
/// Completely policy-free: produces candidates without pre-filtering based on
/// perceived usefulness. No candidate is dropped at enumeration time — rejection
/// happens only in the constraint engine. Equivalent candidates are deduplicated
/// by the pipeline before ranking.
///
/// Custom enumerators can restrict or expand the candidate set (e.g. to limit
/// enumeration to a specific resolution list on embedded targets) without altering
/// constraint or ranking logic.
pub trait CandidateEnumerator {
    /// Iterator type yielding candidate configurations.
    type Iter<'a>: Iterator<Item = CandidateConfig<'a>>
    where
        Self: 'a;

    /// Enumerates all candidate configurations from the given capability triple.
    fn enumerate<'a>(
        &'a self,
        sink: &'a SinkCapabilities,
        source: &'a SourceCapabilities,
        cable: &'a CableCapabilities,
    ) -> Self::Iter<'a>;
}

/// Lazy iterator over the Cartesian product of modes × encodings × depths × FRL rates × DSC.
///
/// Advances like an odometer: the rightmost (innermost) dimension changes fastest.
/// All state is stored inline — no heap allocation.
#[derive(Debug)]
pub struct EnumeratorIter<'a> {
    modes: &'a [VideoMode],
    encodings: [ColorFormat; 4],
    enc_len: usize,
    depths: [[ColorBitDepth; 4]; 4],
    dep_lens: [usize; 4],
    frl_rates: [HdmiForumFrl; 7],
    frl_len: usize,
    dsc: [bool; 2],
    dsc_len: usize,

    mode_idx: usize,
    enc_idx: usize,
    dep_idx: usize,
    frl_idx: usize,
    dsc_idx: usize,
}

impl<'a> Iterator for EnumeratorIter<'a> {
    type Item = CandidateConfig<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.mode_idx >= self.modes.len() {
            return None;
        }

        let candidate = CandidateConfig {
            mode: &self.modes[self.mode_idx],
            color_encoding: self.encodings[self.enc_idx],
            bit_depth: self.depths[self.enc_idx][self.dep_idx],
            frl_rate: self.frl_rates[self.frl_idx],
            dsc_enabled: self.dsc[self.dsc_idx],
        };

        // Advance odometer, innermost (dsc) first.
        self.dsc_idx += 1;
        if self.dsc_idx < self.dsc_len {
            return Some(candidate);
        }
        self.dsc_idx = 0;

        self.frl_idx += 1;
        if self.frl_idx < self.frl_len {
            return Some(candidate);
        }
        self.frl_idx = 0;

        self.dep_idx += 1;
        if self.dep_idx < self.dep_lens[self.enc_idx] {
            return Some(candidate);
        }
        self.dep_idx = 0;

        self.enc_idx += 1;
        if self.enc_idx < self.enc_len {
            return Some(candidate);
        }
        self.enc_idx = 0;

        self.mode_idx += 1;
        Some(candidate)
    }
}

/// Constructs an [`EnumeratorIter`] from a mode slice and capability triple.
fn build_iter<'a>(
    modes: &'a [VideoMode],
    sink: &SinkCapabilities,
    source: &SourceCapabilities,
    cable: &CableCapabilities,
) -> EnumeratorIter<'a> {
    const ALL_FORMATS: [ColorFormat; 4] = [
        ColorFormat::Rgb444,
        ColorFormat::YCbCr444,
        ColorFormat::YCbCr422,
        ColorFormat::YCbCr420,
    ];
    const ALL_DEPTHS: [ColorBitDepth; 4] = [
        ColorBitDepth::Depth8,
        ColorBitDepth::Depth10,
        ColorBitDepth::Depth12,
        ColorBitDepth::Depth16,
    ];
    const ALL_FRL_DESC: [HdmiForumFrl; 7] = [
        HdmiForumFrl::Rate12Gbps4Lanes,
        HdmiForumFrl::Rate10Gbps4Lanes,
        HdmiForumFrl::Rate8Gbps4Lanes,
        HdmiForumFrl::Rate6Gbps4Lanes,
        HdmiForumFrl::Rate6Gbps3Lanes,
        HdmiForumFrl::Rate3Gbps3Lanes,
        HdmiForumFrl::NotSupported,
    ];

    // Color encodings: include those that have at least one depth in ALL_DEPTHS.
    let mut encodings = [ColorFormat::Rgb444; 4];
    let mut enc_len = 0usize;
    for &fmt in &ALL_FORMATS {
        let supported = sink.color_capabilities.for_format(fmt);
        if ALL_DEPTHS.iter().any(|&d| supported.supports(d)) {
            encodings[enc_len] = fmt;
            enc_len += 1;
        }
    }

    // Bit depths per encoding.
    let mut depths = [[ColorBitDepth::Depth8; 4]; 4];
    let mut dep_lens = [0usize; 4];
    for i in 0..enc_len {
        let supported = sink.color_capabilities.for_format(encodings[i]);
        let mut d = 0usize;
        for &depth in &ALL_DEPTHS {
            if supported.supports(depth) {
                depths[i][d] = depth;
                d += 1;
            }
        }
        dep_lens[i] = d;
    }

    // FRL rates: highest qualifying tier first; NotSupported (TMDS) always included.
    let sink_frl_ceil = sink
        .hdmi_forum
        .as_ref()
        .map_or(HdmiForumFrl::NotSupported, |hf| hf.max_frl_rate);
    let effective_ceil = sink_frl_ceil
        .min(source.max_frl_rate)
        .min(cable.max_frl_rate);
    let mut frl_rates = [HdmiForumFrl::NotSupported; 7];
    let mut frl_len = 0usize;
    for &rate in &ALL_FRL_DESC {
        if rate <= effective_ceil {
            frl_rates[frl_len] = rate;
            frl_len += 1;
        }
    }

    // DSC: include true only when both source and sink support DSC 1.2.
    let dsc_supported = source.dsc.is_some_and(|d| d.dsc_1p2)
        && sink
            .hdmi_forum
            .as_ref()
            .is_some_and(|hf| hf.dsc.as_ref().is_some_and(|d| d.dsc_1p2));
    let dsc_len = if dsc_supported { 2 } else { 1 };

    // If no usable encoding exists, set the exhausted sentinel immediately.
    let mode_idx = if enc_len == 0 { modes.len() } else { 0 };

    EnumeratorIter {
        modes,
        encodings,
        enc_len,
        depths,
        dep_lens,
        frl_rates,
        frl_len,
        dsc: [false, true],
        dsc_len,
        mode_idx,
        enc_idx: 0,
        dep_idx: 0,
        frl_idx: 0,
        dsc_idx: 0,
    }
}

/// Candidate enumerator backed by a caller-supplied mode slice.
///
/// The caller provides the mode list at construction time. This is the right
/// choice for embedded targets and for tests that want a controlled mode set.
///
/// ```rust,ignore
/// let enumerator = SliceEnumerator::new(sink.supported_modes.as_slice());
/// ```
#[derive(Debug)]
pub struct SliceEnumerator<'modes> {
    modes: &'modes [VideoMode],
}

impl<'modes> SliceEnumerator<'modes> {
    /// Creates a new enumerator over the given mode slice.
    pub fn new(modes: &'modes [VideoMode]) -> Self {
        Self { modes }
    }
}

impl<'modes> CandidateEnumerator for SliceEnumerator<'modes> {
    type Iter<'a>
        = EnumeratorIter<'a>
    where
        Self: 'a;

    fn enumerate<'a>(
        &'a self,
        sink: &'a SinkCapabilities,
        source: &'a SourceCapabilities,
        cable: &'a CableCapabilities,
    ) -> Self::Iter<'a> {
        build_iter(self.modes, sink, source, cable)
    }
}

/// Default candidate enumerator.
///
/// Generates the full Cartesian product of supported modes, color encodings,
/// bit depths, and FRL tiers implied by the capability triple.
#[derive(Debug, Default)]
pub struct DefaultEnumerator;

impl CandidateEnumerator for DefaultEnumerator {
    type Iter<'a> = EnumeratorIter<'a>;

    fn enumerate<'a>(
        &'a self,
        sink: &'a SinkCapabilities,
        source: &'a SourceCapabilities,
        cable: &'a CableCapabilities,
    ) -> Self::Iter<'a> {
        build_iter(sink.supported_modes.as_slice(), sink, source, cable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use display_types::cea861::{HdmiDscMaxSlices, HdmiForumDsc, HdmiForumSinkCap};
    use display_types::{ColorBitDepths, VideoMode};

    fn mode(refresh_rate: u8) -> VideoMode {
        VideoMode::new(1920, 1080, refresh_rate, false)
    }

    /// Minimal `HdmiForumSinkCap` with just an FRL ceiling set.
    fn hf_sink(max_frl_rate: HdmiForumFrl) -> HdmiForumSinkCap {
        HdmiForumSinkCap::new(
            1,
            0,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            max_frl_rate,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            None,
            None,
            None,
        )
    }

    /// `HdmiForumSinkCap` with an FRL ceiling and DSC 1.2 support.
    fn hf_sink_dsc(max_frl_rate: HdmiForumFrl) -> HdmiForumSinkCap {
        let dsc = HdmiForumDsc::new(
            true,
            false,
            false,
            false,
            false,
            false,
            false,
            HdmiForumFrl::NotSupported,
            HdmiDscMaxSlices::Slices4,
            0,
        );
        HdmiForumSinkCap::new(
            1,
            0,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            max_frl_rate,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            None,
            None,
            Some(dsc),
        )
    }

    fn rgb8_sink() -> SinkCapabilities {
        let mut caps = display_types::ColorCapabilities::default();
        caps.rgb444 = ColorBitDepths::BPC_8;
        SinkCapabilities {
            color_capabilities: caps,
            ..Default::default()
        }
    }

    fn frl6_sink() -> SinkCapabilities {
        let mut caps = display_types::ColorCapabilities::default();
        caps.rgb444 = ColorBitDepths::BPC_8;
        SinkCapabilities {
            color_capabilities: caps,
            hdmi_forum: Some(hf_sink(HdmiForumFrl::Rate6Gbps4Lanes)),
            ..Default::default()
        }
    }

    fn frl6_source() -> SourceCapabilities {
        SourceCapabilities {
            max_frl_rate: HdmiForumFrl::Rate6Gbps4Lanes,
            ..Default::default()
        }
    }

    fn dsc_sink() -> SinkCapabilities {
        let mut caps = display_types::ColorCapabilities::default();
        caps.rgb444 = ColorBitDepths::BPC_8;
        SinkCapabilities {
            color_capabilities: caps,
            hdmi_forum: Some(hf_sink_dsc(HdmiForumFrl::Rate6Gbps4Lanes)),
            ..Default::default()
        }
    }

    fn dsc_source() -> SourceCapabilities {
        use crate::types::source::DscCapabilities;
        SourceCapabilities {
            max_frl_rate: HdmiForumFrl::Rate6Gbps4Lanes,
            dsc: Some(DscCapabilities {
                dsc_1p2: true,
                max_slices: 4,
                max_bpp_x16: 128,
            }),
            ..Default::default()
        }
    }

    fn collect_from<'a>(
        modes: &'a [VideoMode],
        sink: &'a SinkCapabilities,
        source: &'a SourceCapabilities,
        cable: &'a CableCapabilities,
    ) -> alloc::vec::Vec<CandidateConfig<'a>> {
        build_iter(modes, sink, source, cable).collect()
    }

    #[test]
    fn empty_mode_list_yields_nothing() {
        let sink = rgb8_sink();
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::default();
        assert!(collect_from(&[], &sink, &source, &cable).is_empty());
    }

    #[test]
    fn no_usable_encoding_yields_nothing() {
        let modes = [mode(60)];
        let sink = SinkCapabilities::default();
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::default();
        assert!(collect_from(&modes, &sink, &source, &cable).is_empty());
    }

    #[test]
    fn tmds_only_single_mode_rgb8() {
        let modes = [mode(60)];
        let sink = rgb8_sink();
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::default();
        let candidates = collect_from(&modes, &sink, &source, &cable);
        // 1 mode × 1 encoding × 1 depth × 1 FRL (NotSupported) × 1 DSC (false)
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].color_encoding, ColorFormat::Rgb444);
        assert_eq!(candidates[0].bit_depth, ColorBitDepth::Depth8);
        assert_eq!(candidates[0].frl_rate, HdmiForumFrl::NotSupported);
        assert!(!candidates[0].dsc_enabled);
    }

    #[test]
    fn frl_rates_highest_first() {
        // RGB 8bpc, FRL ceiling = Rate6Gbps4Lanes → 4 tiers (6g4l, 6g3l, 3g3l, NotSupported).
        let modes = [mode(60)];
        let sink = frl6_sink();
        let source = frl6_source();
        let cable = CableCapabilities::unconstrained();
        let candidates = collect_from(&modes, &sink, &source, &cable);
        let frl_rates: alloc::vec::Vec<_> = candidates.iter().map(|c| c.frl_rate).collect();
        for i in 1..frl_rates.len() {
            assert!(
                frl_rates[i - 1] >= frl_rates[i],
                "expected descending FRL order at position {i}: {:?} < {:?}",
                frl_rates[i - 1],
                frl_rates[i],
            );
        }
    }

    #[test]
    fn dsc_false_before_true() {
        let modes = [mode(60)];
        let sink = dsc_sink();
        let source = dsc_source();
        let cable = CableCapabilities::unconstrained();
        let candidates = collect_from(&modes, &sink, &source, &cable);
        assert!(
            !candidates[0].dsc_enabled,
            "first candidate should have dsc=false"
        );
        assert!(
            candidates[1].dsc_enabled,
            "second candidate should have dsc=true"
        );
    }

    #[test]
    fn dsc_absent_when_source_lacks_it() {
        let modes = [mode(60)];
        let sink = dsc_sink();
        let source = frl6_source();
        let cable = CableCapabilities::unconstrained();
        let candidates = collect_from(&modes, &sink, &source, &cable);
        assert!(candidates.iter().all(|c| !c.dsc_enabled));
    }

    #[test]
    fn dsc_absent_when_sink_lacks_it() {
        let modes = [mode(60)];
        let sink = frl6_sink();
        let source = dsc_source();
        let cable = CableCapabilities::unconstrained();
        let candidates = collect_from(&modes, &sink, &source, &cable);
        assert!(candidates.iter().all(|c| !c.dsc_enabled));
    }

    #[test]
    fn candidate_count_matches_product() {
        // RGB only, 8+10 bpc, FRL ceiling = Rate6Gbps4Lanes (4 tiers), no DSC.
        // Expected: 2 modes × 1 encoding × 2 depths × 4 FRL rates × 1 DSC = 16.
        let mut caps = display_types::ColorCapabilities::default();
        caps.rgb444 = ColorBitDepths::BPC_8.with(ColorBitDepth::Depth10);
        let sink = SinkCapabilities {
            color_capabilities: caps,
            hdmi_forum: Some(hf_sink(HdmiForumFrl::Rate6Gbps4Lanes)),
            ..Default::default()
        };
        let modes = [mode(60), mode(30)];
        let source = frl6_source();
        let cable = CableCapabilities::unconstrained();
        let candidates = collect_from(&modes, &sink, &source, &cable);
        assert_eq!(candidates.len(), 16);
    }

    // --- build_iter pre-filtering ---

    #[test]
    fn no_hf_scdb_yields_only_tmds() {
        // A sink with no hdmi_forum block has no FRL support; only NotSupported should appear.
        let modes = [mode(60)];
        let sink = rgb8_sink(); // no hdmi_forum
        let source = frl6_source();
        let cable = CableCapabilities::unconstrained();
        let candidates = collect_from(&modes, &sink, &source, &cable);
        assert!(
            candidates
                .iter()
                .all(|c| c.frl_rate == HdmiForumFrl::NotSupported),
            "expected only TMDS candidates when sink has no HF-SCDB"
        );
    }

    #[test]
    fn cable_is_binding_frl_ceiling() {
        // Source and sink both support Rate6Gbps4Lanes, but cable only Rate3Gbps3Lanes.
        let modes = [mode(60)];
        let sink = frl6_sink();
        let source = frl6_source();
        let cable = CableCapabilities {
            max_frl_rate: HdmiForumFrl::Rate3Gbps3Lanes,
            ..CableCapabilities::unconstrained()
        };
        let candidates = collect_from(&modes, &sink, &source, &cable);
        let max_frl = candidates.iter().map(|c| c.frl_rate).max().unwrap();
        assert_eq!(max_frl, HdmiForumFrl::Rate3Gbps3Lanes);
    }

    #[test]
    fn all_seven_frl_tiers_when_ceiling_is_max() {
        // When effective ceiling is Rate12Gbps4Lanes, all 7 tiers should be emitted.
        let mut caps = display_types::ColorCapabilities::default();
        caps.rgb444 = ColorBitDepths::BPC_8;
        let sink = SinkCapabilities {
            color_capabilities: caps,
            hdmi_forum: Some(hf_sink(HdmiForumFrl::Rate12Gbps4Lanes)),
            ..Default::default()
        };
        let source = SourceCapabilities {
            max_frl_rate: HdmiForumFrl::Rate12Gbps4Lanes,
            ..Default::default()
        };
        let modes = [mode(60)];
        let cable = CableCapabilities::unconstrained();
        let candidates = collect_from(&modes, &sink, &source, &cable);
        let mut frl_rates: alloc::vec::Vec<_> = candidates.iter().map(|c| c.frl_rate).collect();
        frl_rates.dedup();
        assert_eq!(frl_rates.len(), 7);
    }

    #[test]
    fn multiple_encodings_all_emitted() {
        // Sink with RGB and YCbCr420 should produce candidates for both.
        let mut caps = display_types::ColorCapabilities::default();
        caps.rgb444 = ColorBitDepths::BPC_8;
        caps.ycbcr420 = ColorBitDepths::BPC_8;
        let sink = SinkCapabilities {
            color_capabilities: caps,
            ..Default::default()
        };
        let modes = [mode(60)];
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::default();
        let candidates = collect_from(&modes, &sink, &source, &cable);
        let has_rgb = candidates
            .iter()
            .any(|c| c.color_encoding == ColorFormat::Rgb444);
        let has_y420 = candidates
            .iter()
            .any(|c| c.color_encoding == ColorFormat::YCbCr420);
        assert!(has_rgb, "expected RGB candidates");
        assert!(has_y420, "expected YCbCr420 candidates");
    }

    #[test]
    fn per_encoding_depths_are_independent() {
        // RGB supports 8+10 bpc; YCbCr420 supports only 8 bpc.
        // RGB candidates should include Depth10; YCbCr420 candidates should not.
        let mut caps = display_types::ColorCapabilities::default();
        caps.rgb444 = ColorBitDepths::BPC_8.with(ColorBitDepth::Depth10);
        caps.ycbcr420 = ColorBitDepths::BPC_8;
        let sink = SinkCapabilities {
            color_capabilities: caps,
            ..Default::default()
        };
        let modes = [mode(60)];
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::default();
        let candidates = collect_from(&modes, &sink, &source, &cable);

        let rgb_depths: alloc::vec::Vec<_> = candidates
            .iter()
            .filter(|c| c.color_encoding == ColorFormat::Rgb444)
            .map(|c| c.bit_depth)
            .collect();
        let y420_depths: alloc::vec::Vec<_> = candidates
            .iter()
            .filter(|c| c.color_encoding == ColorFormat::YCbCr420)
            .map(|c| c.bit_depth)
            .collect();

        assert!(
            rgb_depths.contains(&ColorBitDepth::Depth10),
            "RGB should include Depth10"
        );
        assert!(
            !y420_depths.contains(&ColorBitDepth::Depth10),
            "YCbCr420 should not include Depth10"
        );
    }

    // --- Odometer ordering ---

    #[test]
    fn odometer_sequence_within_mode() {
        // Minimal case: 1 mode × 1 encoding (RGB) × 2 depths × 2 FRL tiers × 2 DSC states = 8
        // candidates, in exact odometer order (depth slowest, then frl, then dsc fastest).
        use crate::types::source::DscCapabilities;
        let mut caps = display_types::ColorCapabilities::default();
        caps.rgb444 = ColorBitDepths::BPC_8.with(ColorBitDepth::Depth10);
        let sink = SinkCapabilities {
            color_capabilities: caps,
            hdmi_forum: Some(hf_sink_dsc(HdmiForumFrl::Rate3Gbps3Lanes)),
            ..Default::default()
        };
        let source = SourceCapabilities {
            max_frl_rate: HdmiForumFrl::Rate3Gbps3Lanes,
            dsc: Some(DscCapabilities {
                dsc_1p2: true,
                max_slices: 4,
                max_bpp_x16: 128,
            }),
            ..Default::default()
        };
        let modes = [mode(60)];
        let cable = CableCapabilities::unconstrained();
        let candidates = collect_from(&modes, &sink, &source, &cable);

        // Expected: depth changes slower than frl, frl slower than dsc.
        type T = (ColorBitDepth, HdmiForumFrl, bool);
        let expected: &[T] = &[
            (ColorBitDepth::Depth8, HdmiForumFrl::Rate3Gbps3Lanes, false),
            (ColorBitDepth::Depth8, HdmiForumFrl::Rate3Gbps3Lanes, true),
            (ColorBitDepth::Depth8, HdmiForumFrl::NotSupported, false),
            (ColorBitDepth::Depth8, HdmiForumFrl::NotSupported, true),
            (ColorBitDepth::Depth10, HdmiForumFrl::Rate3Gbps3Lanes, false),
            (ColorBitDepth::Depth10, HdmiForumFrl::Rate3Gbps3Lanes, true),
            (ColorBitDepth::Depth10, HdmiForumFrl::NotSupported, false),
            (ColorBitDepth::Depth10, HdmiForumFrl::NotSupported, true),
        ];

        assert_eq!(candidates.len(), expected.len());
        for (i, (c, &(depth, frl, dsc))) in candidates.iter().zip(expected).enumerate() {
            assert_eq!(c.bit_depth, depth, "depth mismatch at {i}");
            assert_eq!(c.frl_rate, frl, "frl mismatch at {i}");
            assert_eq!(c.dsc_enabled, dsc, "dsc mismatch at {i}");
        }
    }

    #[test]
    fn mode_is_slowest_dimension() {
        // All candidates for mode[0] must precede all candidates for mode[1].
        let modes = [mode(60), mode(30)];
        let sink = rgb8_sink();
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::default();
        let candidates = collect_from(&modes, &sink, &source, &cable);
        let switch = candidates
            .windows(2)
            .position(|w| !core::ptr::eq(w[0].mode, w[1].mode));
        if let Some(i) = switch {
            // After the first mode switch there must be no further reference to modes[0].
            assert!(
                candidates[i + 1..]
                    .iter()
                    .all(|c| !core::ptr::eq(c.mode, &modes[0])),
                "mode[0] appeared again after the switch at position {i}"
            );
        }
    }

    #[test]
    fn all_candidates_borrow_from_mode_slice() {
        let modes = [mode(60), mode(30)];
        let sink = rgb8_sink();
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::default();
        let candidates = collect_from(&modes, &sink, &source, &cable);
        let mid = candidates.len() / 2;
        assert!(
            candidates[..mid]
                .iter()
                .all(|c| core::ptr::eq(c.mode, &modes[0]))
        );
        assert!(
            candidates[mid..]
                .iter()
                .all(|c| core::ptr::eq(c.mode, &modes[1]))
        );
    }
}
