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
