//! The `ConstraintRule` trait and `Layered` combinator.

use crate::diagnostic::Diagnostic;
use crate::engine::CheckResult;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

/// A single constraint check — the unit of extensibility for the constraint engine.
///
/// Every [`ConstraintEngine`][crate::engine::ConstraintEngine] is shaped identically,
/// so engines and rules compose cleanly via [`Layered`].
///
/// Custom rules can be injected into the default pipeline without reimplementing
/// HDMI specification logic via
/// [`NegotiatorBuilder::with_extra_rule`][crate::NegotiatorBuilder::with_extra_rule].
pub trait ConstraintRule {
    /// Non-fatal diagnostic type emitted for accepted configurations.
    type Warning: Diagnostic;

    /// Fatal constraint violation type emitted for rejected configurations.
    type Violation: Diagnostic;

    /// Evaluates a single constraint against the supplied capabilities.
    #[cfg(any(feature = "alloc", feature = "std"))]
    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig,
    ) -> CheckResult<Self::Warning, Self::Violation>;

    /// Evaluates a single constraint against the supplied capabilities (no-alloc).
    #[cfg(not(any(feature = "alloc", feature = "std")))]
    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig,
    ) -> CheckResult<Self::Violation>;
}

/// Chains a base engine with an additional rule.
///
/// Both the base and the extra rule must share the same `Warning` and `Violation`
/// types (the default path). For rules with distinct types, use `From` bounds to
/// convert into a common output type.
///
/// Construct via [`NegotiatorBuilder::with_extra_rule`][crate::NegotiatorBuilder::with_extra_rule]
/// rather than directly.
pub struct Layered<Base, Extra> {
    /// The base engine or rule.
    pub base: Base,
    /// The additional rule applied after the base.
    pub extra: Extra,
}
