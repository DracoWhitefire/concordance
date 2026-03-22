//! The `is_config_viable` binary probe function.

use crate::engine::{CheckResult, DefaultConstraintEngine};
use crate::engine::ConstraintEngine;
use crate::output::warning::{Violation, Warning};
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

/// Determines whether a specific configuration is viable for the given capabilities.
///
/// Returns structured violations rather than a boolean, giving the caller enough
/// information to surface specific rejection reasons.
///
/// This is the `no_std`-compatible binary probe. Firmware and embedded consumers
/// that cannot afford allocation or iteration use this function directly. The ranked
/// iterator is built on top of this primitive.
///
/// Callers without cable information may pass [`CableCapabilities::unconstrained()`]
/// to recover the previous optimistic behavior.
pub fn is_config_viable(
    sink: &SinkCapabilities,
    source: &SourceCapabilities,
    cable: &CableCapabilities,
    config: &CandidateConfig,
) -> CheckResult<Warning, Violation> {
    DefaultConstraintEngine.check(sink, source, cable, config)
}
