use crate::diagnostic::Diagnostic;
use crate::engine::rule::ConstraintRule;
use crate::output::warning::Violation;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

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
        let _ = (sink, source, cable, config);
        // TODO: requires config.mode.pixel_clock_khz
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::rule::ConstraintRule;
    use crate::output::warning::Violation;
    use crate::types::{CableCapabilities, CandidateConfig, SourceCapabilities};
    use display_types::cea861::HdmiForumFrl;
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
}
