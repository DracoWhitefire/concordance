//! The `NegotiatedConfig` output type.

use alloc::vec::Vec;

use display_types::ResolvedDisplayConfig;

use crate::output::trace::ReasoningTrace;
use crate::output::warning::Warning;

/// A resolved, accepted configuration produced by the negotiation pipeline.
///
/// `NegotiatedConfig` is a pure data struct — it holds resolved values. Helpers
/// that compute derived results (compatibility checks, ranking utilities, mode
/// filters) are free functions in separate modules, not methods on this struct.
///
/// The hardware-relevant fields (video mode, color encoding, transport, DSC, VRR)
/// are grouped in [`resolved`][NegotiatedConfig::resolved] as a [`ResolvedDisplayConfig`].
/// This lets drivers, InfoFrame encoders, and compositors depend only on `display-types`
/// for the programming interface without a hard dependency on `concordance`.
///
/// Generic over the warning type, defaulting to the built-in [`Warning`].
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct NegotiatedConfig<W = Warning> {
    /// The resolved hardware configuration.
    ///
    /// Contains the video mode, color encoding, transport tier, DSC requirement,
    /// and VRR applicability. VRR (`resolved.vrr_applicable`) is always `false` in
    /// the current release; see the roadmap.
    pub resolved: ResolvedDisplayConfig,

    /// Non-fatal warnings about this accepted configuration.
    pub warnings: Vec<W>,

    /// Reasoning trace recording the decisions made during negotiation.
    pub trace: ReasoningTrace,
}
