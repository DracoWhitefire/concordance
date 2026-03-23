use crate::engine::rule::ConstraintRule;
use crate::output::warning::Violation;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

/// Checks that the requested color encoding is supported by the sink.
pub struct ColorEncodingCheck;

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

/// Checks that the requested bit depth is supported by the sink.
pub struct BitDepthCheck;

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
