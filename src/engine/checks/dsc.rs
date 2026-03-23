use crate::output::warning::Violation;
use crate::types::{CandidateConfig, SinkCapabilities, SourceCapabilities};

pub(in crate::engine) fn check_dsc(
    sink: &SinkCapabilities,
    source: &SourceCapabilities,
    config: &CandidateConfig,
) -> Option<Violation> {
    let _ = (sink, source, config);
    // TODO
    None
}
