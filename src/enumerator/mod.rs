//! Candidate enumerator trait and default implementation.

use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

/// Generates all candidate configurations from the intersection of capabilities.
///
/// Completely policy-free: produces candidates without pre-filtering based on
/// perceived usefulness. No candidate is dropped at enumeration time — rejection
/// happens only in the constraint engine. Equivalent candidates are deduplicated
/// by the pipeline before ranking.
///
/// Custom enumerators can restrict or expand the candidate set (e.g. to limit
/// enumeration to a specific resolution list on embedded targets) without altering
/// constraint or ranking logic.
pub trait CandidateEnumerator {
    /// Iterator type yielding candidate configurations.
    type Iter<'a>: Iterator<Item = CandidateConfig<'a>>
    where
        Self: 'a;

    /// Enumerates all candidate configurations from the given capability triple.
    fn enumerate<'a>(
        &'a self,
        sink: &'a SinkCapabilities,
        source: &'a SourceCapabilities,
        cable: &'a CableCapabilities,
    ) -> Self::Iter<'a>;
}

/// Default candidate enumerator.
///
/// Generates the full Cartesian product of supported modes, color encodings,
/// bit depths, and FRL tiers implied by the capability triple.
#[derive(Debug, Default)]
pub struct DefaultEnumerator;

impl CandidateEnumerator for DefaultEnumerator {
    type Iter<'a> = core::iter::Empty<CandidateConfig<'a>>;

    fn enumerate<'a>(
        &'a self,
        _sink: &'a SinkCapabilities,
        _source: &'a SourceCapabilities,
        _cable: &'a CableCapabilities,
    ) -> Self::Iter<'a> {
        // TODO: implement full candidate enumeration
        core::iter::empty()
    }
}
