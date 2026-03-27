//! Pipeline builder for wiring together the three negotiation components.

use alloc::vec::Vec;

use crate::engine::rule::{ConstraintRule, Layered};
use crate::engine::{ConstraintEngine, DefaultConstraintEngine};
use crate::enumerator::{CandidateEnumerator, DefaultEnumerator};
use crate::output::config::NegotiatedConfig;
use crate::output::rejection::RejectedConfig;
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
/// ```
/// # use concordance::{NegotiatorBuilder, SinkCapabilities, SourceCapabilities, CableCapabilities};
/// # let sink = SinkCapabilities::default();
/// # let source = SourceCapabilities::default();
/// # let cable = CableCapabilities::default();
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
    /// and ranked according to the policy.
    ///
    /// To also capture why each rejected candidate failed, use
    /// [`negotiate_with_log`][Self::negotiate_with_log] instead.
    pub fn negotiate(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
    ) -> Vec<NegotiatedConfig<E::Warning>> {
        let (accepted, _) = self.negotiate_inner(sink, source, cable, false);
        accepted
    }

    /// Runs the negotiation pipeline and returns both the accepted configurations and a
    /// per-candidate rejection log.
    ///
    /// The rejection log contains one [`RejectedConfig`] entry for every candidate that
    /// failed the constraint engine, in enumeration order. Each entry records the same
    /// five fields as [`CandidateConfig`][crate::CandidateConfig] plus the violations that
    /// caused the rejection.
    ///
    /// Use this in diagnostic tools or test harnesses where you need to explain why a
    /// specific mode was excluded. The allocation cost is non-trivial on large mode lists;
    /// prefer [`negotiate`][Self::negotiate] when the rejection detail is not needed.
    pub fn negotiate_with_log(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
    ) -> (
        Vec<NegotiatedConfig<E::Warning>>,
        Vec<RejectedConfig<E::Violation>>,
    ) {
        self.negotiate_inner(sink, source, cable, true)
    }

    fn negotiate_inner(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        collect_rejections: bool,
    ) -> (
        Vec<NegotiatedConfig<E::Warning>>,
        Vec<RejectedConfig<E::Violation>>,
    ) {
        let mut accepted: Vec<NegotiatedConfig<E::Warning>> = Vec::new();
        let mut rejected: Vec<RejectedConfig<E::Violation>> = Vec::new();

        for config in self.enumerator.enumerate(sink, source, cable) {
            match self.engine.check(sink, source, cable, &config) {
                Ok(warnings) => {
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
                Err(violations) => {
                    if collect_rejections {
                        rejected.push(RejectedConfig {
                            mode: config.mode.clone(),
                            color_encoding: config.color_encoding,
                            bit_depth: config.bit_depth,
                            frl_rate: config.frl_rate,
                            dsc_enabled: config.dsc_enabled,
                            violations,
                        });
                    }
                }
            }
        }

        (self.ranker.rank(accepted, &self.policy), rejected)
    }
}

#[cfg(all(test, any(feature = "alloc", feature = "std")))]
mod tests {
    use super::*;
    use crate::engine::{CheckResult, ConstraintEngine};
    use crate::enumerator::SliceEnumerator;
    use crate::output::warning::{Violation, Warning};
    use crate::types::CandidateConfig;
    use display_types::{ColorBitDepths, ColorCapabilities, VideoMode};

    // ── stubs ─────────────────────────────────────────────────────────────────

    /// Engine that accepts every candidate with no warnings.
    struct AcceptAllEngine;

    impl ConstraintEngine for AcceptAllEngine {
        type Warning = Warning;
        type Violation = Violation;

        fn check(
            &self,
            _: &SinkCapabilities,
            _: &SourceCapabilities,
            _: &CableCapabilities,
            _: &CandidateConfig<'_>,
        ) -> CheckResult<Warning, Violation> {
            Ok(alloc::vec::Vec::new())
        }
    }

    /// Engine that rejects every candidate with a fixed violation.
    struct RejectAllEngine;

    impl ConstraintEngine for RejectAllEngine {
        type Warning = Warning;
        type Violation = Violation;

        fn check(
            &self,
            _: &SinkCapabilities,
            _: &SourceCapabilities,
            _: &CableCapabilities,
            _: &CandidateConfig<'_>,
        ) -> CheckResult<Warning, Violation> {
            Err(alloc::vec![Violation::ColorEncodingUnsupported])
        }
    }

    /// Ranker that reverses the accepted list, inverting whatever order the
    /// default ranker would have produced.
    struct ReverseRanker;

    impl ConfigRanker for ReverseRanker {
        type Warning = Warning;

        fn rank(
            &self,
            mut configs: Vec<NegotiatedConfig<Warning>>,
            _: &NegotiationPolicy,
        ) -> Vec<NegotiatedConfig<Warning>> {
            configs.reverse();
            configs
        }
    }

    /// Constraint rule that always produces a violation, regardless of input.
    struct AlwaysRejectRule;

    impl crate::engine::rule::ConstraintRule<Violation> for AlwaysRejectRule {
        fn display_name(&self) -> &'static str {
            "always_reject"
        }

        fn check(
            &self,
            _: &SinkCapabilities,
            _: &SourceCapabilities,
            _: &CableCapabilities,
            _: &CandidateConfig<'_>,
        ) -> Option<Violation> {
            Some(Violation::ColorEncodingUnsupported)
        }
    }

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Sink with RGB 8 bpc and no declared modes (modes come from the enumerator).
    fn rgb8_sink() -> SinkCapabilities {
        let mut caps = ColorCapabilities::default();
        caps.rgb444 = ColorBitDepths::BPC_8;
        SinkCapabilities {
            color_capabilities: caps,
            ..Default::default()
        }
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    /// `with_enumerator` replaces the default enumerator; the custom enumerator's
    /// mode list is used even when the sink has no declared modes.
    #[test]
    fn with_enumerator_overrides_sink_modes() {
        let mode = VideoMode::new(1920, 1080, 60, false);
        let sink = rgb8_sink(); // no supported_modes
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::unconstrained();

        let configs = NegotiatorBuilder::default()
            .with_enumerator(SliceEnumerator::new(&[mode]))
            .negotiate(&sink, &source, &cable);

        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].mode.width, 1920);
    }

    /// `with_engine` replaces the constraint check; an accept-all engine admits a
    /// candidate that the default engine would reject (source TMDS ceiling too low).
    #[test]
    fn with_engine_accept_all_overrides_default_rejection() {
        // 1080p@60 RGB 8 bpc TMDS clock ≈ 136 MHz; source ceiling = 50 MHz → default
        // engine rejects it. AcceptAllEngine bypasses that check entirely.
        let mode = VideoMode::new(1920, 1080, 60, false);
        let sink = rgb8_sink();
        let mut source = SourceCapabilities::default();
        source.max_tmds_clock = 50_000; // 50 MHz — below 1080p@60 8 bpc
        let cable = CableCapabilities::unconstrained();

        let configs = NegotiatorBuilder::default()
            .with_enumerator(SliceEnumerator::new(&[mode]))
            .with_engine(AcceptAllEngine)
            .negotiate(&sink, &source, &cable);

        assert_eq!(
            configs.len(),
            1,
            "AcceptAllEngine must admit candidates the default engine would reject"
        );
    }

    /// `with_engine` replaces the constraint check; a reject-all engine empties
    /// the result even for a configuration that the default engine would accept.
    #[test]
    fn with_engine_replaces_constraint_check() {
        let mode = VideoMode::new(1920, 1080, 60, false);
        let sink = rgb8_sink();
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::unconstrained();

        let configs = NegotiatorBuilder::default()
            .with_enumerator(SliceEnumerator::new(&[mode]))
            .with_engine(RejectAllEngine)
            .negotiate(&sink, &source, &cable);

        assert!(
            configs.is_empty(),
            "RejectAllEngine must eliminate all candidates"
        );
    }

    /// `with_ranker` replaces the ordering step; the output reflects the custom
    /// ranker's order rather than the default policy.
    #[test]
    fn with_ranker_replaces_ordering() {
        // Enumerated in slice order: 4K first, then 1080p.
        // The default BEST_QUALITY ranker also puts 4K first (native resolution).
        // ReverseRanker inverts the accepted list, so 1080p appears first.
        let modes = [
            VideoMode::new(3840, 2160, 60, false),
            VideoMode::new(1920, 1080, 60, false),
        ];
        let sink = rgb8_sink();
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::unconstrained();

        let configs = NegotiatorBuilder::default()
            .with_enumerator(SliceEnumerator::new(&modes))
            .with_ranker(ReverseRanker)
            .negotiate(&sink, &source, &cable);

        assert_eq!(configs.len(), 2);
        assert_eq!(
            configs[0].mode.width, 1920,
            "ReverseRanker must put 1080p first"
        );
        assert_eq!(configs[1].mode.width, 3840);
    }

    /// `with_extra_rule` appends a constraint on top of the default engine;
    /// a rule that always rejects eliminates all candidates.
    #[test]
    fn with_extra_rule_applies_additional_constraint() {
        let mode = VideoMode::new(1920, 1080, 60, false);
        let sink = rgb8_sink();
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::unconstrained();

        let configs = NegotiatorBuilder::default()
            .with_enumerator(SliceEnumerator::new(&[mode]))
            .with_extra_rule(AlwaysRejectRule)
            .negotiate(&sink, &source, &cable);

        assert!(
            configs.is_empty(),
            "AlwaysRejectRule must eliminate all candidates"
        );
    }

    /// `negotiate_with_log` returns rejections alongside accepted configs.
    #[test]
    fn negotiate_with_log_captures_rejections() {
        let mode = VideoMode::new(1920, 1080, 60, false);
        let sink = rgb8_sink(); // only RGB 8 bpc
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::unconstrained();

        // SliceEnumerator × rgb8_sink will enumerate RGB 8 bpc (accepted) and other
        // encodings (rejected). Use RejectAllEngine to force every candidate into the log.
        let (accepted, rejected) = NegotiatorBuilder::default()
            .with_enumerator(SliceEnumerator::new(&[mode]))
            .with_engine(RejectAllEngine)
            .negotiate_with_log(&sink, &source, &cable);

        assert!(accepted.is_empty(), "RejectAllEngine must accept nothing");
        assert!(!rejected.is_empty(), "rejected log must be non-empty");
        // Every entry must carry at least one violation.
        for entry in &rejected {
            assert!(
                !entry.violations.is_empty(),
                "each rejection must carry at least one violation"
            );
            assert!(
                entry
                    .violations
                    .iter()
                    .any(|v| matches!(v, Violation::ColorEncodingUnsupported)),
                "each rejection must carry the engine violation"
            );
        }
    }

    /// `negotiate` (without log) produces the same accepted set as `negotiate_with_log`.
    #[test]
    fn negotiate_and_negotiate_with_log_agree_on_accepted() {
        let mode = VideoMode::new(1920, 1080, 60, false);
        let sink = rgb8_sink();
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::unconstrained();

        let accepted_plain = NegotiatorBuilder::default()
            .with_enumerator(SliceEnumerator::new(&[mode.clone()]))
            .negotiate(&sink, &source, &cable);

        let (accepted_log, _) = NegotiatorBuilder::default()
            .with_enumerator(SliceEnumerator::new(&[mode]))
            .negotiate_with_log(&sink, &source, &cable);

        assert_eq!(
            accepted_plain.len(),
            accepted_log.len(),
            "both methods must agree on the accepted count"
        );
    }

    /// `negotiate` does not allocate a rejection log (smoke: just checks it compiles
    /// and runs without panicking).
    #[test]
    fn negotiate_without_log_does_not_panic() {
        let mode = VideoMode::new(1920, 1080, 60, false);
        let sink = rgb8_sink();
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::unconstrained();
        let _ = NegotiatorBuilder::default()
            .with_enumerator(SliceEnumerator::new(&[mode]))
            .with_engine(RejectAllEngine)
            .negotiate(&sink, &source, &cable);
    }

    /// The pipeline deduplicates candidates that are identical across all five
    /// key fields; supplying the same mode twice yields only one accepted config.
    #[test]
    fn negotiate_dedup_removes_identical_candidates() {
        let mode = VideoMode::new(1920, 1080, 60, false);
        let sink = rgb8_sink();
        let source = SourceCapabilities::default();
        let cable = CableCapabilities::unconstrained();

        let configs = NegotiatorBuilder::default()
            .with_enumerator(SliceEnumerator::new(&[mode.clone(), mode]))
            .negotiate(&sink, &source, &cable);

        assert_eq!(
            configs.len(),
            1,
            "identical candidates must be deduplicated"
        );
    }
}
