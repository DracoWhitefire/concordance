//! Candidate configuration type used as input to the constraint engine.

use display_types::cea861::HdmiForumFrl;
use display_types::{ColorBitDepth, DigitalColorEncoding, VideoMode};

/// A specific configuration candidate to be evaluated by the constraint engine.
///
/// Produced by the [`CandidateEnumerator`][crate::enumerator::CandidateEnumerator] during
/// full pipeline runs, or supplied directly by the caller to [`is_config_viable`][crate::is_config_viable].
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateConfig {
    /// The video mode being evaluated.
    pub mode: VideoMode,

    /// Color encoding format.
    pub color_encoding: DigitalColorEncoding,

    /// Color bit depth per channel.
    pub bit_depth: ColorBitDepth,

    /// FRL rate tier, or [`HdmiForumFrl::NotSupported`] for TMDS transport.
    pub frl_rate: HdmiForumFrl,

    /// Whether Display Stream Compression is applied.
    pub dsc_enabled: bool,
}
