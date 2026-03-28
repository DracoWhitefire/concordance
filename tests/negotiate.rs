//! Integration tests for the concordance HDMI negotiation pipeline.
//!
//! Each test exercises the full `NegotiatorBuilder::default().negotiate()` pipeline
//! end-to-end, from capability construction through constraint checking and ranking.
//!
//! Gated on `alloc`/`std` because the negotiation pipeline is not available on
//! bare-metal no-alloc targets.
#![cfg(any(feature = "alloc", feature = "std"))]

use concordance::ranker::policy::NegotiationPolicy;
use concordance::{
    CableCapabilities, NegotiatorBuilder, SinkCapabilities, SourceCapabilities, SupportedModes,
};
use display_types::cea861::{HdmiForumFrl, HdmiForumSinkCap};
use display_types::{ColorBitDepth, ColorBitDepths, ColorCapabilities, ColorFormat, VideoMode};

// ── helpers ───────────────────────────────────────────────────────────────────

/// Builds a minimal `HdmiForumSinkCap` with only the FRL ceiling and TMDS rate set.
fn hf_sink(max_frl_rate: HdmiForumFrl, max_tmds_rate_mhz: u16) -> HdmiForumSinkCap {
    HdmiForumSinkCap::new(
        1,
        max_tmds_rate_mhz,
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

/// Builds a `SinkCapabilities` with a specific mode list and color map.
fn sink_with_modes(modes: Vec<VideoMode>, color: ColorCapabilities) -> SinkCapabilities {
    let (supported_modes, _) = SupportedModes::from_vec(modes);
    let mut sink = SinkCapabilities::default();
    sink.supported_modes = supported_modes;
    sink.color_capabilities = color;
    sink
}

/// Color capabilities for RGB 8 bpc only.
fn rgb8() -> ColorCapabilities {
    let mut caps = ColorCapabilities::default();
    caps.rgb444 = ColorBitDepths::BPC_8;
    caps
}

/// Color capabilities for RGB 8/10/12 bpc and YCbCr 4:4:4 8/10/12 bpc.
fn rgb_deep_and_ycbcr444() -> ColorCapabilities {
    let deep = ColorBitDepths::BPC_8
        .with(ColorBitDepth::Depth10)
        .with(ColorBitDepth::Depth12);
    let mut caps = ColorCapabilities::default();
    caps.rgb444 = deep;
    caps.ycbcr444 = deep;
    caps
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// A sink with no declared video modes produces no negotiated configurations.
#[test]
fn empty_sink_yields_no_configs() {
    let sink = SinkCapabilities::default();
    let source = SourceCapabilities::default();
    let cable = CableCapabilities::unconstrained();

    let configs = NegotiatorBuilder::default().negotiate(&sink, &source, &cable);

    assert!(
        configs.is_empty(),
        "expected no configs, got {}",
        configs.len()
    );
}

/// A simple TMDS-only setup with one mode and one supported color combination
/// produces exactly one accepted configuration.
#[test]
fn single_tmds_mode_accepted() {
    // Sink: 1080p@60, RGB 8 bpc only, no HF-SCDB (TMDS transport only).
    // Source: default — TMDS only, max_tmds_clock=0 (no source clock limit).
    // No limits anywhere → pixel clock ≈ 136 MHz passes through unchecked.
    let sink = sink_with_modes(vec![VideoMode::new(1920, 1080, 60, false)], rgb8());
    let source = SourceCapabilities::default();
    let cable = CableCapabilities::unconstrained();

    let configs = NegotiatorBuilder::default().negotiate(&sink, &source, &cable);

    assert_eq!(configs.len(), 1, "exactly one config expected");
    let cfg = &configs[0];
    assert_eq!(
        (
            cfg.resolved.mode.width,
            cfg.resolved.mode.height,
            cfg.resolved.mode.refresh_rate
        ),
        (1920, 1080, 60)
    );
    assert_eq!(cfg.resolved.color_encoding, ColorFormat::Rgb444);
    assert_eq!(
        cfg.resolved.frl_rate,
        HdmiForumFrl::NotSupported,
        "expected TMDS transport"
    );
    assert!(!cfg.resolved.dsc_required);
}

/// When the source TMDS clock ceiling is below the pixel clock of every mode,
/// all candidates are rejected and the result is empty.
#[test]
fn source_tmds_ceiling_rejects_all_modes() {
    // 1080p@60 RGB 8 bpc TMDS clock ≈ 136 MHz; source ceiling = 50 MHz → all rejected.
    let sink = sink_with_modes(vec![VideoMode::new(1920, 1080, 60, false)], rgb8());
    let mut source = SourceCapabilities::default();
    source.max_tmds_clock = 50_000; // 50 MHz — below 1080p@60 8 bpc TMDS clock
    let cable = CableCapabilities::unconstrained();

    let configs = NegotiatorBuilder::default().negotiate(&sink, &source, &cable);

    assert!(
        configs.is_empty(),
        "expected no configs, got {}",
        configs.len()
    );
}

/// With a full HDMI 2.1 stack (FRL capable sink, source, and cable), the default
/// BEST_QUALITY policy places the native (highest-resolution) mode first.
#[test]
fn full_hdmi21_native_resolution_ranks_first() {
    // Modes: 1080p@60 (non-native) and 4K@60 (native — larger pixel area).
    let mut sink = sink_with_modes(
        vec![
            VideoMode::new(1920, 1080, 60, false),
            VideoMode::new(3840, 2160, 60, false),
        ],
        rgb8(),
    );
    // HDMI 2.1 HF-SCDB: FRL up to 12G4L; TMDS ceiling 600 MHz.
    sink.hdmi_forum = Some(hf_sink(HdmiForumFrl::Rate12Gbps4Lanes, 600));
    let mut source = SourceCapabilities::default();
    source.max_tmds_clock = 600_000;
    source.max_frl_rate = HdmiForumFrl::Rate12Gbps4Lanes;
    let cable = CableCapabilities::unconstrained();

    let configs = NegotiatorBuilder::default().negotiate(&sink, &source, &cable);

    assert!(!configs.is_empty(), "expected at least one accepted config");
    assert_eq!(
        configs[0].resolved.mode.width, 3840,
        "BEST_QUALITY policy must place native (4K) resolution first"
    );
    assert_eq!(configs[0].resolved.mode.height, 2160);
    // Only RGB was declared in color_capabilities.
    for cfg in &configs {
        assert_eq!(
            cfg.resolved.color_encoding,
            ColorFormat::Rgb444,
            "unexpected encoding {:?}",
            cfg.resolved.color_encoding,
        );
    }
}

/// When the cable's FRL ceiling is lower than the source and sink, no accepted
/// configuration may use an FRL rate above that ceiling.
#[test]
fn cable_frl_ceiling_is_binding_constraint() {
    let mut sink = sink_with_modes(vec![VideoMode::new(1920, 1080, 60, false)], rgb8());
    sink.hdmi_forum = Some(hf_sink(HdmiForumFrl::Rate12Gbps4Lanes, 600));
    let mut source = SourceCapabilities::default();
    source.max_tmds_clock = 600_000;
    source.max_frl_rate = HdmiForumFrl::Rate12Gbps4Lanes;
    // Cable caps FRL at tier 3 (6G 4-lane); source and sink support up to tier 6.
    let mut cable = CableCapabilities::unconstrained();
    cable.max_frl_rate = HdmiForumFrl::Rate6Gbps4Lanes;

    let configs = NegotiatorBuilder::default().negotiate(&sink, &source, &cable);

    assert!(!configs.is_empty());
    for cfg in &configs {
        assert!(
            cfg.resolved.frl_rate <= HdmiForumFrl::Rate6Gbps4Lanes,
            "FRL rate {:?} exceeds the cable ceiling Rate6Gbps4Lanes",
            cfg.resolved.frl_rate,
        );
    }
}

/// BEST_PERFORMANCE policy places the higher-refresh-rate mode first when
/// all other criteria are equal.
#[test]
fn performance_policy_ranks_high_refresh_first() {
    // Two 1080p modes with different refresh rates; TMDS only (no FRL complication).
    // Both are "native" since there is no larger mode. High refresh wins as tiebreaker.
    let sink = sink_with_modes(
        vec![
            VideoMode::new(1920, 1080, 60, false),
            VideoMode::new(1920, 1080, 144, false),
        ],
        rgb8(),
    );
    let source = SourceCapabilities::default(); // TMDS only, no clock limit
    let cable = CableCapabilities::unconstrained();

    let configs = NegotiatorBuilder::default()
        .with_policy(NegotiationPolicy::BEST_PERFORMANCE)
        .negotiate(&sink, &source, &cable);

    assert!(!configs.is_empty());
    assert_eq!(
        configs[0].resolved.mode.refresh_rate, 144,
        "BEST_PERFORMANCE should rank 144 Hz before 60 Hz, got {} Hz first",
        configs[0].resolved.mode.refresh_rate,
    );
}

/// With deep-color support, the BEST_QUALITY policy prefers higher bit depth
/// over lower bit depth for the same color format.
#[test]
fn best_quality_prefers_higher_bit_depth() {
    // Sink: single 1080p mode, RGB 8/10/12 bpc.
    // BEST_QUALITY → prefer_color_fidelity → 12 bpc outranks 10 bpc outranks 8 bpc.
    let mut caps = ColorCapabilities::default();
    caps.rgb444 = ColorBitDepths::BPC_8
        .with(ColorBitDepth::Depth10)
        .with(ColorBitDepth::Depth12);
    let sink = sink_with_modes(vec![VideoMode::new(1920, 1080, 60, false)], caps);
    // 1080p@60 12 bpc TMDS clock ≈ 204 MHz (pixel_clock × 6/4); supply 250 MHz headroom.
    let mut source = SourceCapabilities::default();
    source.max_tmds_clock = 250_000;
    let cable = CableCapabilities::unconstrained();

    let configs = NegotiatorBuilder::default()
        .with_policy(NegotiationPolicy::BEST_QUALITY)
        .negotiate(&sink, &source, &cable);

    assert!(!configs.is_empty());
    assert_eq!(
        configs[0].resolved.bit_depth,
        ColorBitDepth::Depth12,
        "BEST_QUALITY should rank 12 bpc first, got {:?}",
        configs[0].resolved.bit_depth,
    );
}

/// When both RGB and YCbCr 4:4:4 are available at the same bit depth, BEST_QUALITY
/// ranks RGB first because it requires no colour-space conversion at the sink.
#[test]
fn best_quality_prefers_rgb_over_ycbcr444_at_same_depth() {
    let sink = sink_with_modes(
        vec![VideoMode::new(1920, 1080, 60, false)],
        rgb_deep_and_ycbcr444(),
    );
    // Supply enough TMDS headroom for all deep-color combinations at 1080p@60.
    let mut source = SourceCapabilities::default();
    source.max_tmds_clock = 300_000;
    let cable = CableCapabilities::unconstrained();

    let configs = NegotiatorBuilder::default()
        .with_policy(NegotiationPolicy::BEST_QUALITY)
        .negotiate(&sink, &source, &cable);

    assert!(!configs.is_empty());
    assert_eq!(
        configs[0].resolved.color_encoding,
        ColorFormat::Rgb444,
        "BEST_QUALITY should prefer RGB over YCbCr 4:4:4; got {:?}",
        configs[0].resolved.color_encoding,
    );
}
