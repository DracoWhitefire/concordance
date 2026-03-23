use crate::diagnostic::Diagnostic;
use crate::engine::rule::ConstraintRule;
use crate::output::warning::Violation;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

/// Checks DSC consistency: if DSC is enabled, both sink and source must declare support.
pub struct DscCheck;

impl<V: Diagnostic + From<Violation>> ConstraintRule<V> for DscCheck {
    fn display_name(&self) -> &'static str {
        "dsc"
    }

    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        _cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> Option<V> {
        let _ = (sink, source, config);
        // TODO
        None
    }
}
