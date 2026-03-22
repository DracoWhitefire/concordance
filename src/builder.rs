//! Pipeline builder for wiring together the three negotiation components.

use alloc::vec::Vec;

use crate::engine::{ConstraintEngine, DefaultConstraintEngine};
use crate::enumerator::{CandidateEnumerator, DefaultEnumerator};
use crate::output::config::NegotiatedConfig;
use crate::output::warning::Warning;
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
pub struct NegotiatorBuilder<
    E = DefaultConstraintEngine,
    En = DefaultEnumerator,
    R = DefaultRanker,
> {
    engine: E,
    enumerator: En,
    ranker: R,
    policy: NegotiationPolicy,
}

impl Default for NegotiatorBuilder {
    fn default() -> Self {
        Self {
            engine: DefaultConstraintEngine,
            enumerator: DefaultEnumerator,
            ranker: DefaultRanker,
            policy: NegotiationPolicy::default(),
        }
    }
}

impl<E, En, R> NegotiatorBuilder<E, En, R>
where
    E: ConstraintEngine,
    En: CandidateEnumerator,
    R: ConfigRanker<Warning = Warning>,
{
    /// Overrides the constraint engine.
    pub fn with_engine<E2: ConstraintEngine>(
        self,
        engine: E2,
    ) -> NegotiatorBuilder<E2, En, R> {
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
    pub fn with_ranker<R2: ConfigRanker<Warning = Warning>>(
        self,
        ranker: R2,
    ) -> NegotiatorBuilder<E, En, R2> {
        NegotiatorBuilder {
            engine: self.engine,
            enumerator: self.enumerator,
            ranker,
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
    R: ConfigRanker<Warning = Warning>,
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
    ) -> Vec<NegotiatedConfig<Warning>> {
        // TODO: implement full pipeline: enumerate → check → deduplicate → rank
        let _candidates = self.enumerator.enumerate(sink, source, cable);
        Vec::new()
    }
}
