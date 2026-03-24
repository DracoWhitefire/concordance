use crate::diagnostic::Diagnostic;
use crate::engine::rule::ConstraintRule;
use crate::output::warning::Violation;
use crate::types::{CableCapabilities, CandidateConfig, SinkCapabilities, SourceCapabilities};

/// Checks that the requested color encoding is supported by the sink at any bit depth.
pub struct ColorEncodingCheck;

impl<V: Diagnostic + From<Violation>> ConstraintRule<V> for ColorEncodingCheck {
    fn display_name(&self) -> &'static str {
        "color_encoding"
    }

    fn check(
        &self,
        sink: &SinkCapabilities,
        _source: &SourceCapabilities,
        _cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> Option<V> {
        if sink
            .color_capabilities
            .for_format(config.color_encoding)
            .is_empty()
        {
            Some(Violation::ColorEncodingUnsupported.into())
        } else {
            None
        }
    }
}

/// Checks that the requested bit depth is supported by the sink for the requested color encoding.
///
/// Note: `BitDepthCheck` does not gate on encoding support. Run [`ColorEncodingCheck`] first
/// so that an unsupported format is rejected before the depth check runs.
pub struct BitDepthCheck;

impl<V: Diagnostic + From<Violation>> ConstraintRule<V> for BitDepthCheck {
    fn display_name(&self) -> &'static str {
        "bit_depth"
    }

    fn check(
        &self,
        sink: &SinkCapabilities,
        _source: &SourceCapabilities,
        _cable: &CableCapabilities,
        config: &CandidateConfig<'_>,
    ) -> Option<V> {
        if !sink
            .color_capabilities
            .for_format(config.color_encoding)
            .supports(config.bit_depth)
        {
            Some(Violation::BitDepthUnsupported.into())
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
    use crate::types::{CableCapabilities, CandidateConfig, SourceCapabilities};
    use display_types::cea861::HdmiForumFrl;
    use display_types::{ColorBitDepth, ColorBitDepths, ColorCapabilities, ColorFormat, VideoMode};

    fn mode() -> VideoMode {
        VideoMode::new(1920, 1080, 60, false)
    }

    fn config<'a>(
        mode: &'a VideoMode,
        encoding: ColorFormat,
        depth: ColorBitDepth,
    ) -> CandidateConfig<'a> {
        CandidateConfig {
            mode,
            color_encoding: encoding,
            bit_depth: depth,
            frl_rate: HdmiForumFrl::NotSupported,
            dsc_enabled: false,
        }
    }

    fn check_encoding(sink: &SinkCapabilities, encoding: ColorFormat) -> Option<Violation> {
        let m = mode();
        ConstraintRule::<Violation>::check(
            &ColorEncodingCheck,
            sink,
            &SourceCapabilities::default(),
            &CableCapabilities::default(),
            &config(&m, encoding, ColorBitDepth::Depth8),
        )
    }

    fn check_depth(
        sink: &SinkCapabilities,
        encoding: ColorFormat,
        depth: ColorBitDepth,
    ) -> Option<Violation> {
        let m = mode();
        ConstraintRule::<Violation>::check(
            &BitDepthCheck,
            sink,
            &SourceCapabilities::default(),
            &CableCapabilities::default(),
            &config(&m, encoding, depth),
        )
    }

    fn rgb_only_sink() -> SinkCapabilities {
        let mut caps = ColorCapabilities::default();
        caps.rgb444 = ColorBitDepths::BPC_8;
        SinkCapabilities {
            color_capabilities: caps,
            ..Default::default()
        }
    }

    fn deep_color_sink() -> SinkCapabilities {
        let mut caps = ColorCapabilities::default();
        caps.rgb444 = ColorBitDepths::BPC_8
            .with(ColorBitDepth::Depth10)
            .with(ColorBitDepth::Depth12);
        caps.ycbcr444 = ColorBitDepths::BPC_8.with(ColorBitDepth::Depth10);
        caps.ycbcr422 = ColorBitDepths::BPC_8;
        caps.ycbcr420 = ColorBitDepths::BPC_8.with(ColorBitDepth::Depth10);
        SinkCapabilities {
            color_capabilities: caps,
            ..Default::default()
        }
    }

    // --- ColorEncodingCheck ---

    #[test]
    fn supported_encoding_passes() {
        assert!(check_encoding(&rgb_only_sink(), ColorFormat::Rgb444).is_none());
    }

    #[test]
    fn unsupported_encoding_rejected() {
        assert!(matches!(
            check_encoding(&rgb_only_sink(), ColorFormat::YCbCr444),
            Some(Violation::ColorEncodingUnsupported)
        ));
    }

    #[test]
    fn all_formats_pass_on_full_sink() {
        let sink = deep_color_sink();
        assert!(check_encoding(&sink, ColorFormat::Rgb444).is_none());
        assert!(check_encoding(&sink, ColorFormat::YCbCr444).is_none());
        assert!(check_encoding(&sink, ColorFormat::YCbCr422).is_none());
        assert!(check_encoding(&sink, ColorFormat::YCbCr420).is_none());
    }

    // --- BitDepthCheck ---

    #[test]
    fn supported_depth_passes() {
        assert!(
            check_depth(
                &deep_color_sink(),
                ColorFormat::Rgb444,
                ColorBitDepth::Depth10
            )
            .is_none()
        );
    }

    #[test]
    fn unsupported_depth_rejected() {
        assert!(matches!(
            check_depth(
                &deep_color_sink(),
                ColorFormat::Rgb444,
                ColorBitDepth::Depth16
            ),
            Some(Violation::BitDepthUnsupported)
        ));
    }

    #[test]
    fn depth_checked_per_format() {
        let sink = deep_color_sink();
        // 10 bpc is supported for RGB and YCbCr 4:4:4 but not 4:2:2.
        assert!(check_depth(&sink, ColorFormat::Rgb444, ColorBitDepth::Depth10).is_none());
        assert!(check_depth(&sink, ColorFormat::YCbCr444, ColorBitDepth::Depth10).is_none());
        assert!(matches!(
            check_depth(&sink, ColorFormat::YCbCr422, ColorBitDepth::Depth10),
            Some(Violation::BitDepthUnsupported)
        ));
    }
}
