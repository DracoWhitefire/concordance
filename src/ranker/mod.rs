//! Configuration ranker trait and default implementation.

pub mod policy;

use alloc::vec::Vec;

use display_types::{ColorFormat, VideoMode};

use crate::diagnostic::Diagnostic;
use crate::output::config::NegotiatedConfig;
use crate::ranker::policy::NegotiationPolicy;

pub use policy::NegotiationPolicy as Policy;

/// Orders validated configurations according to a [`NegotiationPolicy`].
///
/// The default policy encodes a sensible preference (native resolution, max color
/// fidelity, then refresh rate, then fallback formats), but the caller can supply
/// an override via [`NegotiatorBuilder`][crate::NegotiatorBuilder].
pub trait ConfigRanker {
    /// Non-fatal diagnostic type attached to ranked configurations.
    type Warning: Diagnostic;

    /// Ranks and returns the validated configurations in priority order.
    fn rank(
        &self,
        configs: Vec<NegotiatedConfig<Self::Warning>>,
        policy: &NegotiationPolicy,
    ) -> Vec<NegotiatedConfig<Self::Warning>>;
}

/// Default configuration ranker.
///
/// Implements the built-in preference ordering: native resolution, maximum color
/// fidelity, highest refresh rate, then fallback formats. DSC configurations are
/// ranked lower by default.
#[derive(Debug, Default)]
pub struct DefaultRanker;

/// Returns the pixel area of a video mode (`width × height`).
///
/// Used to identify the native resolution of a display: the mode with the greatest pixel
/// area in the accepted set is treated as native.
fn pixel_area(mode: &VideoMode) -> u32 {
    mode.width as u32 * mode.height as u32
}

/// Returns a quality rank for a color encoding format (higher = better fidelity).
///
/// `Rgb444` ranks above `YCbCr444` at the same chroma resolution because it requires no
/// color-space conversion at the sink. In power-saving mode the caller inverts this value
/// to prefer simpler (lower-bandwidth) formats instead.
fn color_format_quality(fmt: ColorFormat) -> u8 {
    match fmt {
        ColorFormat::Rgb444 => 3,
        ColorFormat::YCbCr444 => 2,
        ColorFormat::YCbCr422 => 1,
        ColorFormat::YCbCr420 => 0,
        // Non-exhaustive: treat any future variant as lowest quality.
        _ => 0,
    }
}

impl ConfigRanker for DefaultRanker {
    type Warning = crate::output::warning::Warning;

    fn rank(
        &self,
        configs: Vec<NegotiatedConfig<Self::Warning>>,
        _policy: &NegotiationPolicy,
    ) -> Vec<NegotiatedConfig<Self::Warning>> {
        // TODO: implement ranking according to policy
        configs
    }
}

#[cfg(test)]
mod tests {
    use display_types::{ColorFormat, VideoMode};

    use super::{color_format_quality, pixel_area};

    fn mode(width: u16, height: u16) -> VideoMode {
        VideoMode::new(width, height, 60, false)
    }

    #[test]
    fn color_format_quality_ordering() {
        // Full ordering: Rgb444 > YCbCr444 > YCbCr422 > YCbCr420.
        assert!(color_format_quality(ColorFormat::Rgb444) > color_format_quality(ColorFormat::YCbCr444));
        assert!(color_format_quality(ColorFormat::YCbCr444) > color_format_quality(ColorFormat::YCbCr422));
        assert!(color_format_quality(ColorFormat::YCbCr422) > color_format_quality(ColorFormat::YCbCr420));
    }

    #[test]
    fn color_format_quality_exact_values() {
        assert_eq!(color_format_quality(ColorFormat::Rgb444), 3);
        assert_eq!(color_format_quality(ColorFormat::YCbCr444), 2);
        assert_eq!(color_format_quality(ColorFormat::YCbCr422), 1);
        assert_eq!(color_format_quality(ColorFormat::YCbCr420), 0);
    }

    #[test]
    fn pixel_area_multiplies_width_and_height() {
        assert_eq!(pixel_area(&mode(1920, 1080)), 1920 * 1080);
        assert_eq!(pixel_area(&mode(3840, 2160)), 3840 * 2160);
    }

    #[test]
    fn pixel_area_zero_dimension() {
        assert_eq!(pixel_area(&mode(0, 1080)), 0);
        assert_eq!(pixel_area(&mode(1920, 0)), 0);
    }

    #[test]
    fn pixel_area_does_not_overflow_u32() {
        // 65535 × 65535 = 4_294_836_225, which fits in u32 (max 4_294_967_295).
        let area = pixel_area(&mode(u16::MAX, u16::MAX));
        assert_eq!(area, u16::MAX as u32 * u16::MAX as u32);
    }
}
