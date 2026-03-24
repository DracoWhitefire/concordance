use display_types::cea861::HdmiForumFrl;

use crate::diagnostic::Diagnostic;
use crate::engine::rule::ConstraintRule;
use crate::output::warning::Violation;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

/// Maps an `HdmiForumFrl` rate to a comparable tier index.
///
/// Higher values represent higher bandwidth tiers. Unknown future values
/// (from `#[non_exhaustive]` additions) map to `0` — treated conservatively
/// as unsupported rather than panicking.
pub(in crate::engine) fn frl_tier(rate: HdmiForumFrl) -> u8 {
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
pub struct FrlCeilingCheck;

impl<V: Diagnostic + From<Violation>> ConstraintRule<V> for FrlCeilingCheck {
    fn display_name(&self) -> &'static str {
        "frl_ceiling"
    }

    fn check(
        &self,
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> Option<V> {
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
            Some(Violation::FrlRateExceeded.into())
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::rule::ConstraintRule;
    use crate::output::warning::Violation;
    use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};
    use display_types::VideoMode;
    use display_types::cea861::{HdmiForumFrl, HdmiForumSinkCap};

    fn mode() -> VideoMode {
        VideoMode::new(3840, 2160, 60, false)
    }

    fn config<'a>(mode: &'a VideoMode, frl_rate: HdmiForumFrl) -> CandidateConfig<'a> {
        CandidateConfig {
            mode,
            color_encoding: display_types::ColorFormat::Rgb444,
            bit_depth: display_types::ColorBitDepth::Depth8,
            frl_rate,
            dsc_enabled: false,
        }
    }

    fn hf_sink(max_frl_rate: HdmiForumFrl) -> HdmiForumSinkCap {
        HdmiForumSinkCap::new(
            1,
            0,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            max_frl_rate,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            None,
            None,
            None,
        )
    }

    fn sink_with_frl(max_frl_rate: HdmiForumFrl) -> SinkCapabilities {
        SinkCapabilities {
            hdmi_forum: Some(hf_sink(max_frl_rate)),
            ..Default::default()
        }
    }

    fn source_with_frl(max_frl_rate: HdmiForumFrl) -> SourceCapabilities {
        SourceCapabilities {
            max_frl_rate,
            ..Default::default()
        }
    }

    fn cable_with_frl(max_frl_rate: HdmiForumFrl) -> CableCapabilities {
        CableCapabilities {
            max_frl_rate,
            ..CableCapabilities::unconstrained()
        }
    }

    fn check(
        sink: &SinkCapabilities,
        source: &SourceCapabilities,
        cable: &CableCapabilities,
        frl_rate: HdmiForumFrl,
    ) -> Option<Violation> {
        let m = mode();
        ConstraintRule::<Violation>::check(
            &FrlCeilingCheck,
            sink,
            source,
            cable,
            &config(&m, frl_rate),
        )
    }

    #[test]
    fn tmds_candidate_skips_check() {
        // NotSupported means TMDS transport; FRL ceiling is irrelevant.
        let sink = SinkCapabilities::default(); // no HF-SCDB → ceiling would be 0
        assert!(
            check(
                &sink,
                &SourceCapabilities::default(),
                &CableCapabilities::default(),
                HdmiForumFrl::NotSupported
            )
            .is_none()
        );
    }

    #[test]
    fn frl_within_all_ceilings_passes() {
        let sink = sink_with_frl(HdmiForumFrl::Rate12Gbps4Lanes);
        let source = source_with_frl(HdmiForumFrl::Rate12Gbps4Lanes);
        assert!(
            check(
                &sink,
                &source,
                &CableCapabilities::unconstrained(),
                HdmiForumFrl::Rate6Gbps4Lanes
            )
            .is_none()
        );
    }

    #[test]
    fn frl_exceeds_sink_ceiling_rejected() {
        let sink = sink_with_frl(HdmiForumFrl::Rate6Gbps3Lanes);
        let source = source_with_frl(HdmiForumFrl::Rate12Gbps4Lanes);
        assert!(matches!(
            check(
                &sink,
                &source,
                &CableCapabilities::unconstrained(),
                HdmiForumFrl::Rate8Gbps4Lanes
            ),
            Some(Violation::FrlRateExceeded)
        ));
    }

    #[test]
    fn frl_exceeds_source_ceiling_rejected() {
        let sink = sink_with_frl(HdmiForumFrl::Rate12Gbps4Lanes);
        let source = source_with_frl(HdmiForumFrl::Rate6Gbps3Lanes);
        assert!(matches!(
            check(
                &sink,
                &source,
                &CableCapabilities::unconstrained(),
                HdmiForumFrl::Rate8Gbps4Lanes
            ),
            Some(Violation::FrlRateExceeded)
        ));
    }

    #[test]
    fn frl_exceeds_cable_ceiling_rejected() {
        let sink = sink_with_frl(HdmiForumFrl::Rate12Gbps4Lanes);
        let source = source_with_frl(HdmiForumFrl::Rate12Gbps4Lanes);
        let cable = cable_with_frl(HdmiForumFrl::Rate6Gbps4Lanes);
        assert!(matches!(
            check(&sink, &source, &cable, HdmiForumFrl::Rate10Gbps4Lanes),
            Some(Violation::FrlRateExceeded)
        ));
    }

    #[test]
    fn no_hf_scdb_rejects_any_frl_request() {
        // Sink without HF-SCDB has effective FRL ceiling of 0.
        let sink = SinkCapabilities::default();
        let source = source_with_frl(HdmiForumFrl::Rate12Gbps4Lanes);
        assert!(matches!(
            check(
                &sink,
                &source,
                &CableCapabilities::unconstrained(),
                HdmiForumFrl::Rate3Gbps3Lanes
            ),
            Some(Violation::FrlRateExceeded)
        ));
    }

    #[test]
    fn cable_is_binding_constraint() {
        // Sink and source support 12G; cable only supports 6G 4-lane.
        // A request for 8G must be rejected.
        let sink = sink_with_frl(HdmiForumFrl::Rate12Gbps4Lanes);
        let source = source_with_frl(HdmiForumFrl::Rate12Gbps4Lanes);
        let cable = cable_with_frl(HdmiForumFrl::Rate6Gbps4Lanes);
        assert!(matches!(
            check(&sink, &source, &cable, HdmiForumFrl::Rate8Gbps4Lanes),
            Some(Violation::FrlRateExceeded)
        ));
        // But 6G 4-lane itself is fine.
        assert!(check(&sink, &source, &cable, HdmiForumFrl::Rate6Gbps4Lanes).is_none());
    }
}
