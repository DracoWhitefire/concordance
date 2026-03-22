//! Constraint engine trait and default implementation.

use crate::diagnostic::Diagnostic;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

#[cfg(any(feature = "alloc", feature = "std"))]
use alloc::vec::Vec;

/// The result of a constraint check (alloc).
///
/// Returns accumulated warnings on success or accumulated violations on failure.
#[cfg(any(feature = "alloc", feature = "std"))]
pub type CheckResult<W, V> = Result<Vec<W>, Vec<V>>;

/// The result of a constraint check (no-alloc).
///
/// Returns unit on success or the first violation encountered on failure.
#[cfg(not(any(feature = "alloc", feature = "std")))]
pub type CheckResult<V> = Result<(), V>;

/// Determines whether a given configuration is valid for the supplied capabilities.
///
/// Returns structured violations rather than a boolean, so callers can surface
/// specific rejection reasons. The default implementation enforces HDMI specification
/// rules; callers can wrap or replace it to add vendor-specific constraint rules.
pub trait ConstraintEngine {
    /// Non-fatal diagnostic type emitted for accepted configurations.
    type Warning: Diagnostic;

    /// Fatal constraint violation type emitted for rejected configurations.
    type Violation: Diagnostic;

    /// Evaluates a candidate configuration against the supplied capabilities.
    #[cfg(any(feature = "alloc", feature = "std"))]
    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig,
    ) -> CheckResult<Self::Warning, Self::Violation>;

    /// Evaluates a candidate configuration against the supplied capabilities (no-alloc).
    #[cfg(not(any(feature = "alloc", feature = "std")))]
    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig,
    ) -> CheckResult<Self::Violation>;
}

/// Default HDMI specification constraint engine.
///
/// Enforces HDMI specification rules. Can be wrapped or replaced via
/// [`NegotiatorBuilder`][crate::NegotiatorBuilder] to add vendor-specific rules
/// without forking the crate.
#[derive(Debug, Default)]
pub struct DefaultConstraintEngine;

impl ConstraintEngine for DefaultConstraintEngine {
    type Warning = crate::output::warning::Warning;
    type Violation = crate::output::warning::Violation;

    #[cfg(any(feature = "alloc", feature = "std"))]
    fn check(
        &self,
        _sink: &SinkCapabilities,
        _source: &SourceCapabilities,
        _cable: &CableCapabilities,
        _config: &CandidateConfig,
    ) -> CheckResult<Self::Warning, Self::Violation> {
        // TODO: implement HDMI specification constraint checks
        Ok(Vec::new())
    }

    #[cfg(not(any(feature = "alloc", feature = "std")))]
    fn check(
        &self,
        _sink: &SinkCapabilities,
        _source: &SourceCapabilities,
        _cable: &CableCapabilities,
        _config: &CandidateConfig,
    ) -> CheckResult<Self::Violation> {
        // TODO: implement HDMI specification constraint checks
        Ok(())
    }
}
