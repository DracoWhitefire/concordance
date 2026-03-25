//! Pipeline builder for wiring together the three negotiation components.

use alloc::vec::Vec;

use crate::engine::rule::{ConstraintRule, Layered};
use crate::engine::{ConstraintEngine, DefaultConstraintEngine};
use crate::enumerator::{CandidateEnumerator, DefaultEnumerator};
use crate::output::config::NegotiatedConfig;
use crate::output::trace::ReasoningTrace;
use crate::ranker::policy::NegotiationPolicy;
use crate::ranker::{ConfigRanker, DefaultRanker};
use crate::types::{CableCapabilities, SinkCapabilities, SourceCapabilities};

/// Wires the three pipeline components together and drives the negotiation run.
///
/// Default implementations are used for any slot not explicitly configured.
/// Callers can substitute any component without forking the crate.
///
/// # Example
///
/// ```rust,ignore
/// let configs = NegotiatorBuilder::default()
///     .negotiate(&sink, &source, &cable);
/// ```
pub struct NegotiatorBuilder<E = DefaultConstraintEngine, En = DefaultEnumerator, R = DefaultRanker>
{
    engine: E,
    enumerator: En,
    ranker: R,
    policy: NegotiationPolicy,
}

impl Default for NegotiatorBuilder {
    fn default() -> Self {
        Self {
            engine: DefaultConstraintEngine::default(),
            enumerator: DefaultEnumerator,
            ranker: DefaultRanker,
            policy: NegotiationPolicy::default(),
        }
    }
}

impl<E, En, R> NegotiatorBuilder<E, En, R> {
    /// Overrides the constraint engine.
    pub fn with_engine<E2: ConstraintEngine>(self, engine: E2) -> NegotiatorBuilder<E2, En, R> {
        NegotiatorBuilder {
            engine,
            enumerator: self.enumerator,
            ranker: self.ranker,
            policy: self.policy,
        }
    }

    /// Overrides the candidate enumerator.
    pub fn with_enumerator<En2: CandidateEnumerator>(
        self,
        enumerator: En2,
    ) -> NegotiatorBuilder<E, En2, R> {
        NegotiatorBuilder {
            engine: self.engine,
            enumerator,
            ranker: self.ranker,
            policy: self.policy,
        }
    }

    /// Overrides the configuration ranker.
    pub fn with_ranker<R2: ConfigRanker>(self, ranker: R2) -> NegotiatorBuilder<E, En, R2> {
        NegotiatorBuilder {
            engine: self.engine,
            enumerator: self.enumerator,
            ranker,
            policy: self.policy,
        }
    }

    /// Appends an extra constraint rule to the engine without replacing it.
    ///
    /// The rule is evaluated after all built-in checks. In alloc mode,
    /// violations from both the base engine and the extra rule are collected;
    /// in no-alloc mode the engine short-circuits on the first failure, so
    /// the extra rule is only reached if all built-in checks pass.
    pub fn with_extra_rule<X>(self, rule: X) -> NegotiatorBuilder<Layered<E, X>, En, R>
    where
        E: ConstraintEngine,
        X: ConstraintRule<E::Violation>,
    {
        NegotiatorBuilder {
            engine: Layered::new(self.engine, rule),
            enumerator: self.enumerator,
            ranker: self.ranker,
            policy: self.policy,
        }
    }

    /// Overrides the negotiation policy.
    pub fn with_policy(mut self, policy: NegotiationPolicy) -> Self {
        self.policy = policy;
        self
    }
}

impl<E, En, R> NegotiatorBuilder<E, En, R>
where
    E: ConstraintEngine,
    En: CandidateEnumerator,
    R: ConfigRanker<Warning = E::Warning>,
{
    /// Runs the negotiation pipeline and returns a ranked list of viable configurations.
    ///
    /// Candidates are enumerated, validated by the constraint engine, deduplicated,
    /// and ranked according to the policy. Every rejection is recorded in the
    /// reasoning trace of the candidate.
    pub fn negotiate(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
    ) -> Vec<NegotiatedConfig<E::Warning>> {
        let mut accepted: Vec<NegotiatedConfig<E::Warning>> = Vec::new();

        for config in self.enumerator.enumerate(sink, source, cable) {
            let Ok(warnings) = self.engine.check(sink, source, cable, &config) else {
                continue;
            };

            let negotiated = NegotiatedConfig {
                mode: config.mode.clone(),
                color_encoding: config.color_encoding,
                bit_depth: config.bit_depth,
                frl_rate: config.frl_rate,
                dsc_required: config.dsc_enabled,
                vrr_applicable: false,
                warnings,
                trace: ReasoningTrace::new(),
            };

            // O(n²) dedup — candidate lists are small enough that this is acceptable.
            let is_dup = accepted.iter().any(|c| {
                c.mode == negotiated.mode
                    && c.color_encoding == negotiated.color_encoding
                    && c.bit_depth == negotiated.bit_depth
                    && c.frl_rate == negotiated.frl_rate
                    && c.dsc_required == negotiated.dsc_required
            });
            if !is_dup {
                accepted.push(negotiated);
            }
        }

        self.ranker.rank(accepted, &self.policy)
    }
}
