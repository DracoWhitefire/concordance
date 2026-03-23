//! Constraint engine trait and default implementation.

/// Built-in [`ConstraintRule`] implementations and the [`checks::DEFAULT_CHECKS`] list.
pub mod checks;
pub mod rule;

use core::fmt;

use crate::diagnostic::Diagnostic;
use crate::output::warning::Violation;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

#[cfg(any(feature = "alloc", feature = "std"))]
use alloc::vec::Vec;

pub use rule::ConstraintRule;

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
///
/// See [`ConstraintRule`] and [`rule::Layered`] for rule injection without replacing
/// the entire engine.
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
/// Runs the constraint rules in `checks` in declaration order. Defaults to
/// [`checks::DEFAULT_CHECKS`], which enforces the full set of HDMI specification
/// rules. A custom slice can be supplied via [`DefaultConstraintEngine::with_checks`]
/// to add, remove, or reorder rules without replacing the engine entirely.
///
/// To extend rather than replace, prefer
/// [`NegotiatorBuilder::with_extra_rule`][crate::NegotiatorBuilder::with_extra_rule].
pub struct DefaultConstraintEngine {
    checks: &'static [&'static (dyn ConstraintRule<Violation> + Sync)],
}

impl Default for DefaultConstraintEngine {
    fn default() -> Self {
        Self {
            checks: checks::DEFAULT_CHECKS,
        }
    }
}

impl DefaultConstraintEngine {
    /// Constructs a `DefaultConstraintEngine` with a custom check list.
    ///
    /// The slice must be `'static` to support `no_alloc` targets. Check sets
    /// are always compile-time concerns; use a `static` binding:
    ///
    /// ```rust,ignore
    /// use concordance::engine::checks::{FrlCeilingCheck, TmdsClockCheck};
    /// use concordance::engine::rule::ConstraintRule;
    /// use concordance::output::warning::Violation;
    ///
    /// static MY_CHECKS: &[&(dyn ConstraintRule<Violation> + Sync)] =
    ///     &[&FrlCeilingCheck, &TmdsClockCheck];
    ///
    /// let engine = DefaultConstraintEngine::with_checks(MY_CHECKS);
    /// ```
    pub fn with_checks(checks: &'static [&'static (dyn ConstraintRule<Violation> + Sync)]) -> Self {
        Self { checks }
    }
}

/// Formats the engine as an ordered list of rule display names.
impl fmt::Debug for DefaultConstraintEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for rule in self.checks {
            list.entry(&rule.display_name());
        }
        list.finish()
    }
}

impl ConstraintEngine for DefaultConstraintEngine {
    type Warning = crate::output::warning::Warning;
    type Violation = Violation;

    #[cfg(any(feature = "alloc", feature = "std"))]
    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig,
    ) -> CheckResult<Self::Warning, Self::Violation> {
        let mut violations = Vec::new();

        for rule in self.checks {
            if let Some(v) = rule.check(sink, source, cable, config) {
                violations.push(v);
            }
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
        for rule in self.checks {
            if let Some(v) = rule.check(sink, source, cable, config) {
                return Err(v);
            }
        }
        Ok(())
    }
}
