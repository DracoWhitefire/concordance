//! The `ConstraintRule` trait and `Layered` combinator.

use crate::diagnostic::Diagnostic;
use crate::engine::CheckResult;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

#[cfg(any(feature = "alloc", feature = "std"))]
use alloc::vec;

/// A static slice of constraint rules, used as the check list for [`DefaultConstraintEngine`][crate::engine::DefaultConstraintEngine].
///
/// The `'static` bound is required to support no-alloc targets, where the slice
/// must live for the duration of the program. Check sets are always compile-time
/// concerns; declare yours as a `static` item.
pub type CheckList<V> = &'static [&'static (dyn ConstraintRule<V> + Sync)];

/// A single constraint check — the unit of extensibility for the constraint engine.
///
/// Unlike [`ConstraintEngine`][crate::engine::ConstraintEngine], which coordinates a
/// full check pass and may produce both warnings and violations, a `ConstraintRule`
/// evaluates a single constraint and either finds a violation or does not.
///
/// The return type is always `Option<V>` — no alloc/no-alloc split. The split lives
/// at the `ConstraintEngine` level, which decides whether to collect all violations
/// or short-circuit on the first.
///
/// Custom rules sharing the built-in violation type can be injected via
/// [`NegotiatorBuilder::with_extra_rule`][crate::NegotiatorBuilder::with_extra_rule]
/// without replacing or reimplementing the default engine.
pub trait ConstraintRule<V: Diagnostic> {
    /// A short, stable identifier for this rule.
    ///
    /// Used in [`ReasoningTrace`][crate::output::trace::ReasoningTrace] entries to
    /// identify which rule rejected a candidate. Should be a lowercase snake_case
    /// string (e.g. `"frl_ceiling"`).
    fn display_name(&self) -> &'static str;

    /// Evaluates this rule against the supplied capabilities.
    ///
    /// Returns `Some(violation)` if the candidate fails this constraint, `None` if
    /// it passes.
    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> Option<V>;
}

/// Chains two rules (or a base engine and an extra rule) in sequence.
///
/// Two usage patterns:
///
/// - **`Layered<R1, R2>` where both implement `ConstraintRule<V>`** — composes two
///   rules. Both are evaluated; the first violation found is returned.
///
/// - **`Layered<E, R>` where `E: ConstraintEngine` and `R: ConstraintRule<E::Violation>`**
///   — extends an engine with an additional rule. Implements `ConstraintEngine`,
///   running the base engine then the extra rule.
///
/// Construct via [`NegotiatorBuilder::with_extra_rule`][crate::NegotiatorBuilder::with_extra_rule]
/// rather than directly.
pub struct Layered<Base, Extra> {
    /// The base engine or rule.
    pub base: Base,
    /// The additional rule applied after the base.
    pub extra: Extra,
}

impl<Base, Extra> Layered<Base, Extra> {
    /// Constructs a `Layered` combinator from a base and an extra component.
    pub fn new(base: Base, extra: Extra) -> Self {
        Self { base, extra }
    }
}

/// `Layered<R1, R2>` is itself a `ConstraintRule<V>` when both components are.
///
/// Evaluates the base rule first; if it passes, evaluates the extra rule. Returns
/// the first violation found, or `None` if both pass.
impl<R1, R2, V> ConstraintRule<V> for Layered<R1, R2>
where
    R1: ConstraintRule<V>,
    R2: ConstraintRule<V>,
    V: Diagnostic,
{
    fn display_name(&self) -> &'static str {
        "layered"
    }

    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> Option<V> {
        self.base
            .check(sink, source, cable, config)
            .or_else(|| self.extra.check(sink, source, cable, config))
    }
}

/// `Layered<E, R>` is a `ConstraintEngine` when `E` is an engine and `R` is a rule
/// that produces the same violation type.
///
/// Runs the base engine first, then the extra rule. With alloc, violations from
/// both are collected. Without alloc, the base engine short-circuits on its first
/// violation before the extra rule is reached; any warnings accumulated by the base
/// are propagated through.
impl<E, R> super::ConstraintEngine for Layered<E, R>
where
    E: super::ConstraintEngine,
    R: ConstraintRule<E::Violation>,
{
    type Warning = E::Warning;
    type Violation = E::Violation;

    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> CheckResult<Self::Warning, Self::Violation> {
        #[cfg(any(feature = "alloc", feature = "std"))]
        {
            match self.base.check(sink, source, cable, config) {
                Err(mut violations) => {
                    if let Some(v) = self.extra.check(sink, source, cable, config) {
                        violations.push(v);
                    }
                    Err(violations)
                }
                Ok(warnings) => {
                    if let Some(v) = self.extra.check(sink, source, cable, config) {
                        Err(vec![v])
                    } else {
                        Ok(warnings)
                    }
                }
            }
        }
        #[cfg(not(any(feature = "alloc", feature = "std")))]
        {
            let warnings = self.base.check(sink, source, cable, config)?;
            if let Some(v) = self.extra.check(sink, source, cable, config) {
                return Err(v);
            }
            Ok(warnings)
        }
    }
}
