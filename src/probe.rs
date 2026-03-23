//! The `is_config_viable` binary probe function.

use crate::engine::ConstraintEngine;
use crate::engine::DefaultConstraintEngine;
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
/// On alloc targets, returns all accumulated warnings on success and all violations
/// on failure. On no-alloc targets, returns up to [`crate::engine::MAX_WARNINGS`]
/// warnings on success and the first violation on failure.
///
/// Callers without cable information may pass [`CableCapabilities::unconstrained()`]
/// to recover the previous optimistic behavior.
pub fn is_config_viable(
    sink: &SinkCapabilities,
    source: &SourceCapabilities,
    cable: &CableCapabilities,
    config: &CandidateConfig,
) -> crate::engine::CheckResult<Warning, Violation> {
    DefaultConstraintEngine::default().check(sink, source, cable, config)
}
