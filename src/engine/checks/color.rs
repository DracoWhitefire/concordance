use crate::engine::rule::ConstraintRule;
use crate::output::warning::Violation;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

pub(in crate::engine) struct ColorEncodingCheck;

impl ConstraintRule<Violation> for ColorEncodingCheck {
    fn display_name(&self) -> &'static str {
        "color_encoding"
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

pub(in crate::engine) struct BitDepthCheck;

impl ConstraintRule<Violation> for BitDepthCheck {
    fn display_name(&self) -> &'static str {
        "bit_depth"
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
