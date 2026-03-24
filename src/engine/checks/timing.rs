use crate::diagnostic::Diagnostic;
use crate::engine::rule::ConstraintRule;
use crate::output::warning::Violation;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};
use display_types::pixel_clock_khz_cvt_rb_estimate;

/// Checks that the requested refresh rate falls within the sink's supported range.
pub struct RefreshRateCheck;

impl<V: Diagnostic + From<Violation>> ConstraintRule<V> for RefreshRateCheck {
    fn display_name(&self) -> &'static str {
        "refresh_rate_range"
    }

    fn check(
        &self,
        sink: &SinkCapabilities,
        _source: &SourceCapabilities,
        _cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> Option<V> {
        let min_hz = sink.min_v_rate?;
        let max_hz = sink.max_v_rate?;
        let rate_hz = config.mode.refresh_rate as u16;
        if rate_hz < min_hz || rate_hz > max_hz {
            Some(
                Violation::RefreshRateOutOfRange {
                    rate_hz,
                    min_hz,
                    max_hz,
                }
                .into(),
            )
        } else {
            None
        }
    }
}

/// Checks that the TMDS character rate does not exceed the ceiling for the link.
pub struct TmdsClockCheck;

impl<V: Diagnostic + From<Violation>> ConstraintRule<V> for TmdsClockCheck {
    fn display_name(&self) -> &'static str {
        "tmds_clock_ceiling"
    }

    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> Option<V> {
        // Skip FRL candidates — TMDS clock is not relevant for FRL links.
        use display_types::cea861::HdmiForumFrl;
        if config.frl_rate != HdmiForumFrl::NotSupported {
            return None;
        }

        let pixel_clock_khz = pixel_clock_khz_cvt_rb_estimate(config.mode);

        // TMDS character clock depends on color encoding and bit depth.
        // YCbCr 4:2:0 halves the pixel rate; deep color multiplies it.
        let encoding_denom = match config.color_encoding {
            display_types::ColorFormat::YCbCr420 => 2u32,
            _ => 1u32,
        };
        let depth_numer = match config.bit_depth {
            display_types::ColorBitDepth::Depth10 => 5u32,
            display_types::ColorBitDepth::Depth12 => 6u32,
            display_types::ColorBitDepth::Depth16 => 8u32,
            _ => 4u32, // 8 bpc: clock × 1
        };
        // tmds_clock = pixel_clock × depth_numer / (4 × encoding_denom)
        let tmds_khz = pixel_clock_khz * depth_numer / (4 * encoding_denom);

        // Find the binding ceiling across sink, source, and cable.
        let sink_limit = sink
            .hdmi_vsdb
            .as_ref()
            .and_then(|v| v.max_tmds_clock_mhz)
            .map(|mhz| mhz as u32 * 1000)
            .unwrap_or(u32::MAX);
        let source_limit = if source.max_tmds_clock > 0 {
            source.max_tmds_clock
        } else {
            u32::MAX
        };
        let cable_limit = if cable.max_tmds_clock > 0 {
            cable.max_tmds_clock
        } else {
            u32::MAX
        };
        let limit_khz = sink_limit.min(source_limit).min(cable_limit);

        if limit_khz == u32::MAX || tmds_khz <= limit_khz {
            None
        } else {
            Some(
                Violation::PixelClockExceeded {
                    required_mhz: tmds_khz / 1000,
                    limit_mhz: limit_khz / 1000,
                }
                .into(),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::rule::ConstraintRule;
    use crate::output::warning::Violation;
    use crate::types::{CableCapabilities, CandidateConfig, SourceCapabilities};
    use display_types::cea861::{HdmiForumFrl, HdmiVsdb, HdmiVsdbFlags};
    use display_types::{ColorBitDepth, ColorFormat, VideoMode};

    fn mode(refresh_rate: u8) -> VideoMode {
        VideoMode::new(1920, 1080, refresh_rate, false)
    }

    fn config(mode: &VideoMode) -> CandidateConfig<'_> {
        CandidateConfig {
            mode,
            color_encoding: ColorFormat::Rgb444,
            bit_depth: ColorBitDepth::Depth8,
            frl_rate: HdmiForumFrl::NotSupported,
            dsc_enabled: false,
        }
    }

    fn tmds_check(
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        mode: &VideoMode,
        encoding: ColorFormat,
        depth: ColorBitDepth,
        frl_rate: HdmiForumFrl,
    ) -> Option<Violation> {
        ConstraintRule::<Violation>::check(
            &TmdsClockCheck,
            sink,
            source,
            cable,
            &CandidateConfig {
                mode,
                color_encoding: encoding,
                bit_depth: depth,
                frl_rate,
                dsc_enabled: false,
            },
        )
    }

    fn sink_with_tmds_limit(max_mhz: u16) -> SinkCapabilities {
        SinkCapabilities {
            hdmi_vsdb: Some(HdmiVsdb::new(
                0,
                HdmiVsdbFlags::empty(),
                Some(max_mhz),
                None,
                None,
                None,
                None,
            )),
            ..Default::default()
        }
    }

    fn source_with_tmds_limit(max_khz: u32) -> SourceCapabilities {
        SourceCapabilities {
            max_tmds_clock: max_khz,
            ..Default::default()
        }
    }

    fn check(sink: &SinkCapabilities, mode: &VideoMode) -> Option<Violation> {
        ConstraintRule::<Violation>::check(
            &RefreshRateCheck,
            sink,
            &SourceCapabilities::default(),
            &CableCapabilities::default(),
            &config(mode),
        )
    }

    #[test]
    fn within_range_passes() {
        let sink = SinkCapabilities {
            min_v_rate: Some(24),
            max_v_rate: Some(144),
            ..Default::default()
        };
        assert!(check(&sink, &mode(60)).is_none());
    }

    #[test]
    fn above_max_rejected() {
        let sink = SinkCapabilities {
            min_v_rate: Some(24),
            max_v_rate: Some(60),
            ..Default::default()
        };
        assert!(matches!(
            check(&sink, &mode(144)),
            Some(Violation::RefreshRateOutOfRange {
                rate_hz: 144,
                min_hz: 24,
                max_hz: 60
            })
        ));
    }

    #[test]
    fn below_min_rejected() {
        let sink = SinkCapabilities {
            min_v_rate: Some(48),
            max_v_rate: Some(144),
            ..Default::default()
        };
        assert!(matches!(
            check(&sink, &mode(24)),
            Some(Violation::RefreshRateOutOfRange {
                rate_hz: 24,
                min_hz: 48,
                max_hz: 144
            })
        ));
    }

    #[test]
    fn no_bounds_skips_check() {
        assert!(check(&SinkCapabilities::default(), &mode(240)).is_none());
    }

    #[test]
    fn at_boundary_passes() {
        let sink = SinkCapabilities {
            min_v_rate: Some(24),
            max_v_rate: Some(144),
            ..Default::default()
        };
        assert!(check(&sink, &mode(24)).is_none());
        assert!(check(&sink, &mode(144)).is_none());
    }

    // --- TmdsClockCheck tests ---
    // CVT-RB estimate for 1920×1080@60: (1920+160)×(1080+8)×60/1000 = 135_782 kHz ≈ 136 MHz.
    // At 8bpc RGB, TMDS clock == pixel clock (depth_numer=4, denom=4).
    // At 10bpc RGB, TMDS clock = pixel_clock × 5/4 ≈ 169_727 kHz ≈ 170 MHz.
    // At YCbCr420 8bpc, TMDS clock = pixel_clock / 2 ≈ 67_891 kHz ≈ 68 MHz.

    #[test]
    fn frl_candidate_skips_tmds_check() {
        // Any FRL rate bypasses the TMDS check regardless of limits.
        let sink = sink_with_tmds_limit(10); // absurdly low — would fail if checked
        let m = mode(60);
        assert!(
            tmds_check(
                &sink,
                &SourceCapabilities::default(),
                &CableCapabilities::default(),
                &m,
                ColorFormat::Rgb444,
                ColorBitDepth::Depth8,
                HdmiForumFrl::Rate3Gbps3Lanes,
            )
            .is_none()
        );
    }

    #[test]
    fn no_limits_passes() {
        // Default sink (no VSDB), default source (max_tmds_clock=0), unconstrained cable.
        let m = mode(60);
        assert!(
            tmds_check(
                &SinkCapabilities::default(),
                &SourceCapabilities::default(),
                &CableCapabilities::default(),
                &m,
                ColorFormat::Rgb444,
                ColorBitDepth::Depth8,
                HdmiForumFrl::NotSupported,
            )
            .is_none()
        );
    }

    #[test]
    fn within_sink_limit_passes() {
        // 165 MHz sink limit; 1080p60 8bpc ≈ 136 MHz — passes.
        let sink = sink_with_tmds_limit(165);
        let m = mode(60);
        assert!(
            tmds_check(
                &sink,
                &SourceCapabilities::default(),
                &CableCapabilities::default(),
                &m,
                ColorFormat::Rgb444,
                ColorBitDepth::Depth8,
                HdmiForumFrl::NotSupported,
            )
            .is_none()
        );
    }

    #[test]
    fn exceeds_sink_limit_rejected() {
        // 100 MHz sink limit; 1080p60 8bpc ≈ 136 MHz — fails.
        let sink = sink_with_tmds_limit(100);
        let m = mode(60);
        assert!(matches!(
            tmds_check(
                &sink,
                &SourceCapabilities::default(),
                &CableCapabilities::default(),
                &m,
                ColorFormat::Rgb444,
                ColorBitDepth::Depth8,
                HdmiForumFrl::NotSupported,
            ),
            Some(Violation::PixelClockExceeded { limit_mhz: 100, .. })
        ));
    }

    #[test]
    fn exceeds_source_limit_rejected() {
        // Source limited to 100_000 kHz; 1080p60 8bpc ≈ 135_782 kHz — fails.
        let source = source_with_tmds_limit(100_000);
        let m = mode(60);
        assert!(matches!(
            tmds_check(
                &SinkCapabilities::default(),
                &source,
                &CableCapabilities::default(),
                &m,
                ColorFormat::Rgb444,
                ColorBitDepth::Depth8,
                HdmiForumFrl::NotSupported,
            ),
            Some(Violation::PixelClockExceeded { limit_mhz: 100, .. })
        ));
    }

    #[test]
    fn deep_color_10bpc_increases_clock() {
        // At 165 MHz, 8bpc passes (≈136 MHz) but 10bpc fails (≈170 MHz).
        let sink = sink_with_tmds_limit(165);
        let m = mode(60);
        assert!(
            tmds_check(
                &sink,
                &SourceCapabilities::default(),
                &CableCapabilities::default(),
                &m,
                ColorFormat::Rgb444,
                ColorBitDepth::Depth8,
                HdmiForumFrl::NotSupported,
            )
            .is_none()
        );
        assert!(matches!(
            tmds_check(
                &sink,
                &SourceCapabilities::default(),
                &CableCapabilities::default(),
                &m,
                ColorFormat::Rgb444,
                ColorBitDepth::Depth10,
                HdmiForumFrl::NotSupported,
            ),
            Some(Violation::PixelClockExceeded { limit_mhz: 165, .. })
        ));
    }

    #[test]
    fn ycbcr420_halves_clock_stays_within_limit() {
        // 80 MHz limit; YCbCr420 8bpc ≈ 68 MHz — passes.
        // Same mode with RGB 8bpc ≈ 136 MHz would fail.
        let sink = sink_with_tmds_limit(80);
        let m = mode(60);
        assert!(matches!(
            tmds_check(
                &sink,
                &SourceCapabilities::default(),
                &CableCapabilities::default(),
                &m,
                ColorFormat::Rgb444,
                ColorBitDepth::Depth8,
                HdmiForumFrl::NotSupported,
            ),
            Some(Violation::PixelClockExceeded { .. })
        ));
        assert!(
            tmds_check(
                &sink,
                &SourceCapabilities::default(),
                &CableCapabilities::default(),
                &m,
                ColorFormat::YCbCr420,
                ColorBitDepth::Depth8,
                HdmiForumFrl::NotSupported,
            )
            .is_none()
        );
    }
}
