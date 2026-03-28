//! Rejection log types for opt-in per-candidate audit output.

use alloc::vec::Vec;

use display_types::cea861::HdmiForumFrl;
use display_types::{ColorBitDepth, ColorFormat, VideoMode};

use crate::output::warning::Violation;

/// A candidate configuration that was rejected during negotiation, together with the
/// violations that caused it to fail.
///
/// `RejectedConfig` is produced by
/// [`NegotiatorBuilder::negotiate_with_log`][crate::NegotiatorBuilder::negotiate_with_log].
/// It records the same five fields that [`CandidateConfig`][crate::CandidateConfig] carries,
/// plus the violations returned by the constraint engine.
///
/// Generic over the violation type, defaulting to the built-in [`Violation`].
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct RejectedConfig<V = Violation> {
    /// The video mode of the rejected candidate.
    pub mode: VideoMode,

    /// Color encoding format of the rejected candidate.
    pub color_encoding: ColorFormat,

    /// Color bit depth per channel of the rejected candidate.
    pub bit_depth: ColorBitDepth,

    /// FRL rate tier, or [`HdmiForumFrl::NotSupported`] for TMDS transport.
    pub frl_rate: HdmiForumFrl,

    /// Whether Display Stream Compression was enabled for the rejected candidate.
    pub dsc_enabled: bool,

    /// The violations that caused this candidate to be rejected.
    pub violations: Vec<V>,
}
