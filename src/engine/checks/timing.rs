use crate::output::warning::Violation;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

pub(in crate::engine) fn check_refresh_rate_range(
    sink: &SinkCapabilities,
    config: &CandidateConfig,
) -> Option<Violation> {
    let _ = (sink, config);
    // TODO
    None
}

pub(in crate::engine) fn check_tmds_clock_ceiling(
    sink: &SinkCapabilities,
    source: &SourceCapabilities,
    cable: &CableCapabilities,
    config: &CandidateConfig,
) -> Option<Violation> {
    let _ = (sink, source, cable, config);
    // TODO: requires config.pixel_clock_khz; skip when None
    None
}
