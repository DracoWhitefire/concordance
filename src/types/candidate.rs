//! Candidate configuration type used as input to the constraint engine.

use display_types::cea861::HdmiForumFrl;
use display_types::{ColorBitDepth, ColorFormat, VideoMode};

/// A specific configuration candidate to be evaluated by the constraint engine.
///
/// Produced by the [`CandidateEnumerator`][crate::enumerator::CandidateEnumerator] during
/// full pipeline runs, or supplied directly by the caller to [`is_config_viable`][crate::is_config_viable].
///
/// Borrows the [`VideoMode`] from the sink's mode list rather than owning a copy —
/// most candidates are rejected, so deferring the copy to acceptance time (in
/// [`NegotiatedConfig`][crate::output::config::NegotiatedConfig]) avoids redundant
/// allocation. Pixel clock information is accessed via `config.mode.pixel_clock_khz`.
///
/// # Serde
///
/// Only `Serialize` is derived (behind the `serde` feature flag). Deserialization is not
/// supported for borrowed types; construct `CandidateConfig` programmatically.
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateConfig<'a> {
    /// The video mode being evaluated.
    pub mode: &'a VideoMode,

    /// Color encoding format.
    pub color_encoding: ColorFormat,

    /// Color bit depth per channel.
    pub bit_depth: ColorBitDepth,

    /// FRL rate tier, or [`HdmiForumFrl::NotSupported`] for TMDS transport.
    pub frl_rate: HdmiForumFrl,

    /// Whether Display Stream Compression is applied.
    pub dsc_enabled: bool,
}

impl<'a> CandidateConfig<'a> {
    /// Constructs a `CandidateConfig`.
    pub fn new(
        mode: &'a VideoMode,
        color_encoding: ColorFormat,
        bit_depth: ColorBitDepth,
        frl_rate: HdmiForumFrl,
        dsc_enabled: bool,
    ) -> Self {
        Self {
            mode,
            color_encoding,
            bit_depth,
            frl_rate,
            dsc_enabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use display_types::VideoMode;

    #[test]
    fn new_matches_struct_literal() {
        let mode = VideoMode::new(1920, 1080, 60, false);
        let via_new = CandidateConfig::new(
            &mode,
            ColorFormat::Rgb444,
            ColorBitDepth::Depth8,
            HdmiForumFrl::NotSupported,
            false,
        );
        let via_literal = CandidateConfig {
            mode: &mode,
            color_encoding: ColorFormat::Rgb444,
            bit_depth: ColorBitDepth::Depth8,
            frl_rate: HdmiForumFrl::NotSupported,
            dsc_enabled: false,
        };
        assert_eq!(via_new, via_literal);
    }
}
