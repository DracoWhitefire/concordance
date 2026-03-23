//! Constraint engine trait and default implementation.

use display_types::cea861::HdmiForumFrl;

use crate::diagnostic::Diagnostic;
use crate::output::warning::Violation;
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
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig,
    ) -> CheckResult<Self::Warning, Self::Violation> {
        let mut violations = Vec::new();

        if let Some(v) = check_frl_ceiling(sink, source, cable, config) {
            violations.push(v);
        }

        if violations.is_empty() {
            Ok(Vec::new())
        } else {
            Err(violations)
        }
    }

    #[cfg(not(any(feature = "alloc", feature = "std")))]
    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig,
    ) -> CheckResult<Self::Violation> {
        if let Some(v) = check_frl_ceiling(sink, source, cable, config) {
            return Err(v);
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Individual checks
// ---------------------------------------------------------------------------

/// Maps an `HdmiForumFrl` rate to a comparable tier index.
///
/// Higher values represent higher bandwidth tiers. Unknown future values
/// (from `#[non_exhaustive]` additions) map to `0` — treated conservatively
/// as unsupported rather than panicking.
fn frl_tier(rate: HdmiForumFrl) -> u8 {
    match rate {
        HdmiForumFrl::NotSupported => 0,
        HdmiForumFrl::Rate3Gbps3Lanes => 1,
        HdmiForumFrl::Rate6Gbps3Lanes => 2,
        HdmiForumFrl::Rate6Gbps4Lanes => 3,
        HdmiForumFrl::Rate8Gbps4Lanes => 4,
        HdmiForumFrl::Rate10Gbps4Lanes => 5,
        HdmiForumFrl::Rate12Gbps4Lanes => 6,
        _ => 0,
    }
}

/// Checks that the requested FRL rate does not exceed the ceiling imposed by
/// the sink, source, and cable.
///
/// TMDS candidates (`frl_rate == NotSupported`) are not subject to this check.
/// A sink without an HF-SCDB declares no FRL support and imposes a ceiling of 0.
fn check_frl_ceiling(
    sink: &SinkCapabilities,
    source: &SourceCapabilities,
    cable: &CableCapabilities,
    config: &CandidateConfig,
) -> Option<Violation> {
    if config.frl_rate == HdmiForumFrl::NotSupported {
        return None;
    }

    let requested = frl_tier(config.frl_rate);

    let sink_max = sink
        .hdmi_forum
        .as_ref()
        .map_or(0, |hf| frl_tier(hf.max_frl_rate));
    let source_max = frl_tier(source.max_frl_rate);
    let cable_max = frl_tier(cable.max_frl_rate);

    let effective_max = sink_max.min(source_max).min(cable_max);

    if requested > effective_max {
        Some(Violation::FrlRateExceeded)
    } else {
        None
    }
}
