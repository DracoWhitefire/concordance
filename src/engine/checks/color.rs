use crate::output::warning::Violation;
use crate::types::{CandidateConfig, SinkCapabilities};

pub(in crate::engine) fn check_color_encoding(
    sink: &SinkCapabilities,
    config: &CandidateConfig,
) -> Option<Violation> {
    let _ = (sink, config);
    // TODO
    None
}

pub(in crate::engine) fn check_bit_depth(
    sink: &SinkCapabilities,
    config: &CandidateConfig,
) -> Option<Violation> {
    let _ = (sink, config);
    // TODO
    None
}
