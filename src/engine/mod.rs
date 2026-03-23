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

pub use rule::{CheckList, ConstraintRule};

/// Maximum number of warnings that can be accumulated in a single constraint check
/// on no-alloc targets.
///
/// When the engine accepts a configuration and has collected this many warnings,
/// any further warnings are silently dropped. Increase this constant (and recompile)
/// if more capacity is needed during debugging.
pub const MAX_WARNINGS: usize = 8;

/// The result of a constraint check.
///
/// - **alloc/std**: `Result<Vec<W>, Vec<V>>` — all warnings on success, all violations on failure.
/// - **no-alloc**: `Result<[Option<W>; MAX_WARNINGS], V>` — up to [`MAX_WARNINGS`] warnings on
///   success, the first violation encountered on failure.
#[cfg(any(feature = "alloc", feature = "std"))]
pub type CheckResult<W, V> = Result<Vec<W>, Vec<V>>;

/// The result of a constraint check (no-alloc).
#[cfg(not(any(feature = "alloc", feature = "std")))]
pub type CheckResult<W, V> = Result<[Option<W>; MAX_WARNINGS], V>;

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
    ///
    /// On alloc targets, returns all accumulated warnings on success and all
    /// violations on failure. On no-alloc targets, returns up to
    /// [`MAX_WARNINGS`] warnings on success and the first violation on failure.
    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> CheckResult<Self::Warning, Self::Violation>;
}

/// Default HDMI specification constraint engine.
///
/// Generic over the violation type `V`, which defaults to the built-in [`Violation`]
/// enum. Callers that need a richer violation hierarchy can define their own type
/// and use it here, as long as it implements `From<`[`Violation`]`>`:
///
/// ```rust,ignore
/// #[derive(Debug, Display)]
/// enum MyViolation {
///     Builtin(Violation),
///     HdrCertificationFailed,
/// }
///
/// impl From<Violation> for MyViolation {
///     fn from(v: Violation) -> Self { MyViolation::Builtin(v) }
/// }
///
/// static MY_CHECKS: CheckList<MyViolation> = &[
///     &FrlCeilingCheck, &TmdsClockCheck, /* ... */ &MyHdrCheck,
/// ];
///
/// NegotiatorBuilder::default()
///     .with_engine(DefaultConstraintEngine::<MyViolation>::with_checks(MY_CHECKS))
///     .with_extra_rule(AnotherMyRule)
/// ```
///
/// For the common case — built-in violations only — `DefaultConstraintEngine::default()`
/// uses [`checks::DEFAULT_CHECKS`] and no type annotation is needed.
pub struct DefaultConstraintEngine<V: 'static = Violation> {
    checks: CheckList<V>,
}

/// Uses [`checks::DEFAULT_CHECKS`] as the rule list. Only available for `V = Violation`
/// since `DEFAULT_CHECKS` is typed for the built-in violation set.
impl Default for DefaultConstraintEngine<Violation> {
    fn default() -> Self {
        Self {
            checks: checks::DEFAULT_CHECKS,
        }
    }
}

impl<V: Diagnostic> DefaultConstraintEngine<V> {
    /// Constructs a `DefaultConstraintEngine` with a custom check list.
    ///
    /// The slice must be `'static` to support no-alloc targets. Check sets are
    /// always compile-time concerns; use a `static` binding:
    ///
    /// ```rust,ignore
    /// use concordance::engine::checks::{FrlCeilingCheck, TmdsClockCheck};
    /// use concordance::engine::rule::CheckList;
    /// use concordance::output::warning::Violation;
    ///
    /// static MY_CHECKS: CheckList<Violation> = &[&FrlCeilingCheck, &TmdsClockCheck];
    ///
    /// let engine = DefaultConstraintEngine::with_checks(MY_CHECKS);
    /// ```
    pub fn with_checks(checks: CheckList<V>) -> Self {
        Self { checks }
    }
}

/// Formats the engine as an ordered list of rule display names.
impl<V: Diagnostic> fmt::Debug for DefaultConstraintEngine<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();
        for rule in self.checks {
            list.entry(&rule.display_name());
        }
        list.finish()
    }
}

impl<V: Diagnostic> ConstraintEngine for DefaultConstraintEngine<V> {
    type Warning = crate::output::warning::Warning;
    type Violation = V;

    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> CheckResult<Self::Warning, Self::Violation> {
        #[cfg(any(feature = "alloc", feature = "std"))]
        {
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
        {
            for rule in self.checks {
                if let Some(v) = rule.check(sink, source, cable, config) {
                    return Err(v);
                }
            }
            Ok(Default::default())
        }
    }
}
