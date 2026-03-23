use crate::engine::rule::ConstraintRule;
use crate::output::warning::Violation;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

pub(in crate::engine) struct RefreshRateCheck;

impl ConstraintRule<Violation> for RefreshRateCheck {
    fn name(&self) -> &'static str {
        "refresh_rate_range"
    }

    fn check(
        &self,
        sink: &SinkCapabilities,
        _source: &SourceCapabilities,
        _cable: &CableCapabilities,
        config: &CandidateConfig,
    ) -> Option<Violation> {
        let _ = (sink, config);
        // TODO
        None
    }
}

pub(in crate::engine) struct TmdsClockCheck;

impl ConstraintRule<Violation> for TmdsClockCheck {
    fn name(&self) -> &'static str {
        "tmds_clock_ceiling"
    }

    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig,
    ) -> Option<Violation> {
        let _ = (sink, source, cable, config);
        // TODO: requires config.pixel_clock_khz; skip when None
        None
    }
}
