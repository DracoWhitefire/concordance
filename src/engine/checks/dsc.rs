use crate::engine::rule::ConstraintRule;
use crate::output::warning::Violation;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

pub(in crate::engine) struct DscCheck;

impl ConstraintRule<Violation> for DscCheck {
    fn display_name(&self) -> &'static str {
        "dsc"
    }

    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        _cable: &CableCapabilities,
        config: &CandidateConfig,
    ) -> Option<Violation> {
        let _ = (sink, source, config);
        // TODO
        None
    }
}
