//! The `NegotiatedConfig` output type.

use alloc::vec::Vec;

use display_types::cea861::HdmiForumFrl;
use display_types::{ColorBitDepth, ColorFormat, VideoMode};

use crate::output::trace::ReasoningTrace;
use crate::output::warning::Warning;

/// A resolved, accepted configuration produced by the negotiation pipeline.
///
/// `NegotiatedConfig` is a pure data struct — it holds resolved values. Helpers
/// that compute derived results (compatibility checks, ranking utilities, mode
/// filters) are free functions in separate modules, not methods on this struct.
///
/// Generic over the warning type, defaulting to the built-in [`Warning`].
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct NegotiatedConfig<W = Warning> {
    /// The resolved video mode.
    pub mode: VideoMode,

    /// Color encoding format for this configuration.
    pub color_encoding: ColorFormat,

    /// Color bit depth per channel.
    pub bit_depth: ColorBitDepth,

    /// FRL rate tier, or [`HdmiForumFrl::NotSupported`] for TMDS transport.
    pub frl_rate: HdmiForumFrl,

    /// Whether Display Stream Compression is required for this configuration.
    pub dsc_required: bool,

    /// Whether Variable Refresh Rate is applicable for this configuration.
    ///
    /// Always `false` in the current release. VRR range validation (min/max refresh
    /// from the sink's VRR range descriptor) is not yet implemented. See the roadmap.
    pub vrr_applicable: bool,

    /// Non-fatal warnings about this accepted configuration.
    pub warnings: Vec<W>,

    /// Reasoning trace recording the decisions made during negotiation.
    pub trace: ReasoningTrace,
}
