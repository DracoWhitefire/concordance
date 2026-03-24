use crate::diagnostic::Diagnostic;
use crate::engine::rule::ConstraintRule;
use crate::output::warning::Violation;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

/// Checks that the requested color encoding is supported by the sink at any bit depth.
pub struct ColorEncodingCheck;

impl<V: Diagnostic + From<Violation>> ConstraintRule<V> for ColorEncodingCheck {
    fn display_name(&self) -> &'static str {
        "color_encoding"
    }

    fn check(
        &self,
        sink: &SinkCapabilities,
        _source: &SourceCapabilities,
        _cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> Option<V> {
        if sink
            .color_capabilities
            .for_format(config.color_encoding)
            .is_empty()
        {
            Some(Violation::ColorEncodingUnsupported.into())
        } else {
            None
        }
    }
}

/// Checks that the requested bit depth is supported by the sink for the requested color encoding.
pub struct BitDepthCheck;

impl<V: Diagnostic + From<Violation>> ConstraintRule<V> for BitDepthCheck {
    fn display_name(&self) -> &'static str {
        "bit_depth"
    }

    fn check(
        &self,
        sink: &SinkCapabilities,
        _source: &SourceCapabilities,
        _cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> Option<V> {
        if !sink
            .color_capabilities
            .for_format(config.color_encoding)
            .supports(config.bit_depth)
        {
            Some(Violation::BitDepthUnsupported.into())
        } else {
            None
        }
    }
}
