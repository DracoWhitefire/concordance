use display_types::cea861::HdmiForumFrl;

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
pub(in crate::engine) fn check_frl_ceiling(
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
