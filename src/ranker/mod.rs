//! Configuration ranker trait and default implementation.

pub mod policy;

use alloc::vec::Vec;

use core::cmp::Ordering;

use display_types::{ColorFormat, VideoMode};

use crate::diagnostic::Diagnostic;
use crate::output::config::NegotiatedConfig;
use crate::output::trace::DecisionStep;
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
        // ColorFormat is #[non_exhaustive]; treat any future variant as lowest quality.
        _ => 0,
    }
}

/// Compares two validated configurations according to `policy`.
///
/// Returns [`Ordering::Less`] when `a` should appear before `b` in the ranked output
/// (i.e. `a` is the preferred configuration). Criteria are applied in priority order;
/// the first non-equal result determines the outcome.
fn compare_configs<W>(
    a: &NegotiatedConfig<W>,
    b: &NegotiatedConfig<W>,
    policy: &NegotiationPolicy,
    native_pixels: u32,
) -> Ordering {
    // 1. DSC penalty: non-DSC (false) sorts before DSC (true).
    if policy.penalize_dsc {
        let ord = a.dsc_required.cmp(&b.dsc_required);
        if ord != Ordering::Equal {
            return ord;
        }
    }

    // 2. Native resolution: native sorts before non-native.
    if policy.prefer_native_resolution {
        let a_native = pixel_area(&a.mode) == native_pixels;
        let b_native = pixel_area(&b.mode) == native_pixels;
        // true > false, so reverse (b, a) to put native (true) first.
        let ord = b_native.cmp(&a_native);
        if ord != Ordering::Equal {
            return ord;
        }
    }

    // 3. Quality/performance dimension.
    if policy.prefer_color_fidelity {
        // Bit depth descending, then color format quality descending, then refresh rate descending.
        let ord = b.bit_depth.bits_per_primary().cmp(&a.bit_depth.bits_per_primary());
        if ord != Ordering::Equal {
            return ord;
        }
        let ord = color_format_quality(b.color_encoding).cmp(&color_format_quality(a.color_encoding));
        if ord != Ordering::Equal {
            return ord;
        }
        let ord = b.mode.refresh_rate.cmp(&a.mode.refresh_rate);
        if ord != Ordering::Equal {
            return ord;
        }
    } else if policy.prefer_high_refresh {
        // Refresh rate descending, then bit depth descending, then color format quality descending.
        let ord = b.mode.refresh_rate.cmp(&a.mode.refresh_rate);
        if ord != Ordering::Equal {
            return ord;
        }
        let ord = b.bit_depth.bits_per_primary().cmp(&a.bit_depth.bits_per_primary());
        if ord != Ordering::Equal {
            return ord;
        }
        let ord = color_format_quality(b.color_encoding).cmp(&color_format_quality(a.color_encoding));
        if ord != Ordering::Equal {
            return ord;
        }
    } else {
        // Power saving: lower refresh rate, lower bit depth, and simpler format are preferred.
        let ord = a.mode.refresh_rate.cmp(&b.mode.refresh_rate);
        if ord != Ordering::Equal {
            return ord;
        }
        let ord = a.bit_depth.bits_per_primary().cmp(&b.bit_depth.bits_per_primary());
        if ord != Ordering::Equal {
            return ord;
        }
        let ord = color_format_quality(a.color_encoding).cmp(&color_format_quality(b.color_encoding));
        if ord != Ordering::Equal {
            return ord;
        }
    }

    // 4. Progressive before interlaced: false sorts before true.
    let ord = a.mode.interlaced.cmp(&b.mode.interlaced);
    if ord != Ordering::Equal {
        return ord;
    }

    // 5. Lower FRL rate first: simpler link wins when all else is equal.
    let ord = a.frl_rate.cmp(&b.frl_rate);
    if ord != Ordering::Equal {
        return ord;
    }

    // 6. Larger resolution area first (final tiebreaker).
    pixel_area(&b.mode).cmp(&pixel_area(&a.mode))
}

/// Appends [`DecisionStep::PreferenceApplied`] steps to `config`'s trace for each
/// ranking criterion that applies to this specific configuration.
///
/// These are per-config facts, not relative comparisons. They give a diagnostic tool
/// enough context to explain a configuration's characteristics without requiring
/// knowledge of the full ranked list.
fn record_preferences<W>(
    config: &mut NegotiatedConfig<W>,
    policy: &NegotiationPolicy,
    native_pixels: u32,
) {
    if policy.penalize_dsc && config.dsc_required {
        config.trace.steps.push(DecisionStep::PreferenceApplied {
            rule: "DSC penalized".into(),
        });
    }

    if policy.prefer_native_resolution && pixel_area(&config.mode) == native_pixels {
        config.trace.steps.push(DecisionStep::PreferenceApplied {
            rule: "native resolution preferred".into(),
        });
    }

    let quality_rule = if policy.prefer_color_fidelity {
        "color fidelity preferred"
    } else if policy.prefer_high_refresh {
        "high refresh rate preferred"
    } else {
        "power saving ordering applied"
    };
    config.trace.steps.push(DecisionStep::PreferenceApplied {
        rule: quality_rule.into(),
    });

    if !config.mode.interlaced {
        config.trace.steps.push(DecisionStep::PreferenceApplied {
            rule: "progressive mode preferred".into(),
        });
    }
}

impl ConfigRanker for DefaultRanker {
    type Warning = crate::output::warning::Warning;

    fn rank(
        &self,
        mut configs: Vec<NegotiatedConfig<Self::Warning>>,
        policy: &NegotiationPolicy,
    ) -> Vec<NegotiatedConfig<Self::Warning>> {
        let native_pixels = configs.iter().map(|c| pixel_area(&c.mode)).max().unwrap_or(0);

        configs.sort_by(|a, b| compare_configs(a, b, policy, native_pixels));

        for config in &mut configs {
            record_preferences(config, policy, native_pixels);
        }

        configs
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use core::cmp::Ordering;

    use display_types::cea861::HdmiForumFrl;
    use display_types::{ColorBitDepth, ColorFormat, VideoMode};

    use crate::output::{config::NegotiatedConfig, trace::ReasoningTrace, warning::Warning};
    use crate::ranker::policy::NegotiationPolicy;

    use crate::output::trace::DecisionStep;

    use super::{color_format_quality, compare_configs, pixel_area, record_preferences};

    fn mode(width: u16, height: u16) -> VideoMode {
        VideoMode::new(width, height, 60, false)
    }

    /// Builds a `NegotiatedConfig` with the given fields for use in ranking tests.
    fn config(
        width: u16,
        height: u16,
        refresh_rate: u8,
        interlaced: bool,
        color_encoding: ColorFormat,
        bit_depth: ColorBitDepth,
        frl_rate: HdmiForumFrl,
        dsc_required: bool,
    ) -> NegotiatedConfig<Warning> {
        NegotiatedConfig {
            mode: VideoMode::new(width, height, refresh_rate, interlaced),
            color_encoding,
            bit_depth,
            frl_rate,
            dsc_required,
            vrr_applicable: false,
            warnings: Vec::new(),
            trace: ReasoningTrace::new(),
        }
    }

    /// Shorthand for a typical 1080p60 progressive config with sensible defaults.
    fn base() -> NegotiatedConfig<Warning> {
        config(
            1920, 1080, 60, false,
            ColorFormat::Rgb444, ColorBitDepth::Depth8,
            HdmiForumFrl::NotSupported, false,
        )
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

    // --- compare_configs ---

    const NATIVE: u32 = 1920 * 1080;

    #[test]
    fn dsc_penalized_ranks_lower() {
        let no_dsc = base();
        let with_dsc = NegotiatedConfig { dsc_required: true, ..base() };
        let policy = NegotiationPolicy::BEST_QUALITY; // penalize_dsc = true

        assert_eq!(compare_configs(&no_dsc, &with_dsc, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&with_dsc, &no_dsc, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn dsc_not_penalized_under_performance() {
        // BEST_PERFORMANCE has penalize_dsc = false, so DSC status should not affect order.
        // Make DSC config have higher refresh to confirm it wins on refresh instead.
        let no_dsc = base(); // 60 Hz
        let with_dsc = NegotiatedConfig {
            dsc_required: true,
            mode: VideoMode::new(1920, 1080, 120, false),
            ..base()
        };
        let policy = NegotiationPolicy::BEST_PERFORMANCE;

        // DSC config has higher refresh — it should rank first since DSC is not penalized.
        assert_eq!(compare_configs(&with_dsc, &no_dsc, &policy, NATIVE), Ordering::Less);
    }

    #[test]
    fn native_resolution_ranked_first() {
        let uhd = config(3840, 2160, 60, false, ColorFormat::Rgb444, ColorBitDepth::Depth8, HdmiForumFrl::NotSupported, false);
        let fhd = base();
        let native_pixels = pixel_area(&uhd.mode); // 4K is native
        let policy = NegotiationPolicy::BEST_QUALITY;

        assert_eq!(compare_configs(&uhd, &fhd, &policy, native_pixels), Ordering::Less);
        assert_eq!(compare_configs(&fhd, &uhd, &policy, native_pixels), Ordering::Greater);
    }

    #[test]
    fn native_resolution_equal_area_falls_through_to_next_criterion() {
        // Both configs share the same pixel area, so both are "native" and the
        // native-resolution criterion yields Equal. The next criterion (bit depth
        // under BEST_QUALITY) must then decide the order.
        let depth10 = NegotiatedConfig { bit_depth: ColorBitDepth::Depth10, ..base() };
        let depth8 = base();
        let policy = NegotiationPolicy::BEST_QUALITY; // prefer_native_resolution = true
        let native_pixels = NATIVE; // both configs match

        assert_eq!(compare_configs(&depth10, &depth8, &policy, native_pixels), Ordering::Less);
        assert_eq!(compare_configs(&depth8, &depth10, &policy, native_pixels), Ordering::Greater);
    }

    #[test]
    fn color_fidelity_prefers_higher_depth() {
        let depth10 = NegotiatedConfig { bit_depth: ColorBitDepth::Depth10, ..base() };
        let depth8 = base();
        let policy = NegotiationPolicy::BEST_QUALITY;

        assert_eq!(compare_configs(&depth10, &depth8, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&depth8, &depth10, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn color_fidelity_prefers_rgb_over_ycbcr444() {
        let rgb = base(); // Rgb444
        let ycbcr = NegotiatedConfig { color_encoding: ColorFormat::YCbCr444, ..base() };
        let policy = NegotiationPolicy::BEST_QUALITY;

        assert_eq!(compare_configs(&rgb, &ycbcr, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&ycbcr, &rgb, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn color_fidelity_equal_depth_falls_through_to_format() {
        // Same bit depth; color format quality must break the tie under BEST_QUALITY.
        let rgb = base(); // Rgb444
        let ycbcr = NegotiatedConfig { color_encoding: ColorFormat::YCbCr444, ..base() };
        let policy = NegotiationPolicy::BEST_QUALITY;

        assert_eq!(compare_configs(&rgb, &ycbcr, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&ycbcr, &rgb, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn color_fidelity_equal_depth_equal_format_falls_through_to_refresh() {
        // Same bit depth and color format; refresh rate must break the tie under BEST_QUALITY.
        let hz120 = NegotiatedConfig { mode: VideoMode::new(1920, 1080, 120, false), ..base() };
        let hz60 = base();
        let policy = NegotiationPolicy::BEST_QUALITY;

        assert_eq!(compare_configs(&hz120, &hz60, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&hz60, &hz120, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn color_fidelity_prefers_higher_refresh_as_tiebreak() {
        // Same depth and format; refresh rate breaks the tie under BEST_QUALITY.
        let hz120 = NegotiatedConfig { mode: VideoMode::new(1920, 1080, 120, false), ..base() };
        let hz60 = base();
        let policy = NegotiationPolicy::BEST_QUALITY;

        assert_eq!(compare_configs(&hz120, &hz60, &policy, NATIVE), Ordering::Less);
    }

    #[test]
    fn performance_prefers_high_refresh_over_depth() {
        // 120 Hz at 8-bit should beat 60 Hz at 10-bit under BEST_PERFORMANCE.
        let hz120_8bit = base(); // 60 Hz — swap in 120 Hz below
        let hz120 = NegotiatedConfig {
            mode: VideoMode::new(1920, 1080, 120, false),
            bit_depth: ColorBitDepth::Depth8,
            ..base()
        };
        let hz60_10bit = NegotiatedConfig { bit_depth: ColorBitDepth::Depth10, ..base() };
        let policy = NegotiationPolicy::BEST_PERFORMANCE;
        let _ = hz120_8bit;

        assert_eq!(compare_configs(&hz120, &hz60_10bit, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&hz60_10bit, &hz120, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn performance_equal_refresh_falls_through_to_depth() {
        // Same refresh rate; bit depth must break the tie under BEST_PERFORMANCE.
        let depth10 = NegotiatedConfig { bit_depth: ColorBitDepth::Depth10, ..base() };
        let depth8 = base();
        let policy = NegotiationPolicy::BEST_PERFORMANCE;

        assert_eq!(compare_configs(&depth10, &depth8, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&depth8, &depth10, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn performance_equal_refresh_equal_depth_falls_through_to_format() {
        // Same refresh rate and bit depth; color format quality must break the tie.
        let rgb = base(); // Rgb444
        let ycbcr = NegotiatedConfig { color_encoding: ColorFormat::YCbCr444, ..base() };
        let policy = NegotiationPolicy::BEST_PERFORMANCE;

        assert_eq!(compare_configs(&rgb, &ycbcr, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&ycbcr, &rgb, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn power_saving_prefers_low_refresh() {
        let hz60 = base();
        let hz120 = NegotiatedConfig { mode: VideoMode::new(1920, 1080, 120, false), ..base() };
        let policy = NegotiationPolicy::POWER_SAVING;

        assert_eq!(compare_configs(&hz60, &hz120, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&hz120, &hz60, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn power_saving_prefers_low_depth() {
        let depth8 = base();
        let depth10 = NegotiatedConfig { bit_depth: ColorBitDepth::Depth10, ..base() };
        let policy = NegotiationPolicy::POWER_SAVING;

        assert_eq!(compare_configs(&depth8, &depth10, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&depth10, &depth8, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn power_saving_equal_refresh_falls_through_to_depth() {
        // Same refresh rate; lower bit depth must break the tie under POWER_SAVING.
        let depth8 = base();
        let depth10 = NegotiatedConfig { bit_depth: ColorBitDepth::Depth10, ..base() };
        let policy = NegotiationPolicy::POWER_SAVING;

        assert_eq!(compare_configs(&depth8, &depth10, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&depth10, &depth8, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn power_saving_equal_refresh_equal_depth_falls_through_to_format() {
        // Same refresh rate and bit depth; simpler color format must break the tie under POWER_SAVING.
        let y420 = NegotiatedConfig { color_encoding: ColorFormat::YCbCr420, ..base() };
        let rgb = base();
        let policy = NegotiationPolicy::POWER_SAVING;

        assert_eq!(compare_configs(&y420, &rgb, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&rgb, &y420, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn power_saving_prefers_simpler_format() {
        let y420 = NegotiatedConfig { color_encoding: ColorFormat::YCbCr420, ..base() };
        let rgb = base();
        let policy = NegotiationPolicy::POWER_SAVING;

        assert_eq!(compare_configs(&y420, &rgb, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&rgb, &y420, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn progressive_before_interlaced() {
        let progressive = base();
        let interlaced = NegotiatedConfig {
            mode: VideoMode::new(1920, 1080, 60, true),
            ..base()
        };
        let policy = NegotiationPolicy::BEST_QUALITY;

        assert_eq!(compare_configs(&progressive, &interlaced, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&interlaced, &progressive, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn lower_frl_rate_tiebreak() {
        let low_frl = NegotiatedConfig { frl_rate: HdmiForumFrl::Rate3Gbps3Lanes, ..base() };
        let high_frl = NegotiatedConfig { frl_rate: HdmiForumFrl::Rate6Gbps3Lanes, ..base() };
        let policy = NegotiationPolicy::BEST_QUALITY;

        assert_eq!(compare_configs(&low_frl, &high_frl, &policy, NATIVE), Ordering::Less);
        assert_eq!(compare_configs(&high_frl, &low_frl, &policy, NATIVE), Ordering::Greater);
    }

    #[test]
    fn resolution_area_tiebreak() {
        // 1920×1200 has more pixels than 1920×1080; both are "non-native" (native = 4K here).
        let wider = config(1920, 1200, 60, false, ColorFormat::Rgb444, ColorBitDepth::Depth8, HdmiForumFrl::NotSupported, false);
        let narrower = base();
        let native_pixels = pixel_area(&VideoMode::new(3840, 2160, 60, false)); // 4K is native
        let policy = NegotiationPolicy::BEST_QUALITY;

        assert_eq!(compare_configs(&wider, &narrower, &policy, native_pixels), Ordering::Less);
        assert_eq!(compare_configs(&narrower, &wider, &policy, native_pixels), Ordering::Greater);
    }

    // --- record_preferences ---

    fn has_preference(config: &NegotiatedConfig<Warning>, rule: &str) -> bool {
        config.trace.steps.iter().any(|step| {
            matches!(step, DecisionStep::PreferenceApplied { rule: r } if r == rule)
        })
    }

    #[test]
    fn trace_records_dsc_penalty() {
        let mut c = NegotiatedConfig { dsc_required: true, ..base() };
        record_preferences(&mut c, &NegotiationPolicy::BEST_QUALITY, NATIVE);
        assert!(has_preference(&c, "DSC penalized"));
    }

    #[test]
    fn trace_no_dsc_penalty_when_not_penalized() {
        let mut c = NegotiatedConfig { dsc_required: true, ..base() };
        record_preferences(&mut c, &NegotiationPolicy::BEST_PERFORMANCE, NATIVE); // penalize_dsc=false
        assert!(!has_preference(&c, "DSC penalized"));
    }

    #[test]
    fn trace_no_dsc_penalty_when_not_required() {
        let mut c = base(); // dsc_required=false
        record_preferences(&mut c, &NegotiationPolicy::BEST_QUALITY, NATIVE);
        assert!(!has_preference(&c, "DSC penalized"));
    }

    #[test]
    fn trace_records_native_resolution() {
        let mut c = base();
        record_preferences(&mut c, &NegotiationPolicy::BEST_QUALITY, NATIVE); // pixel_area matches
        assert!(has_preference(&c, "native resolution preferred"));
    }

    #[test]
    fn trace_no_native_resolution_when_not_native() {
        let mut c = base();
        let other_native = pixel_area(&VideoMode::new(3840, 2160, 60, false));
        record_preferences(&mut c, &NegotiationPolicy::BEST_QUALITY, other_native);
        assert!(!has_preference(&c, "native resolution preferred"));
    }

    #[test]
    fn trace_no_native_resolution_when_not_preferred() {
        let mut c = base();
        let policy = NegotiationPolicy { prefer_native_resolution: false, ..NegotiationPolicy::BEST_QUALITY };
        record_preferences(&mut c, &policy, NATIVE);
        assert!(!has_preference(&c, "native resolution preferred"));
    }

    #[test]
    fn trace_records_color_fidelity_preferred() {
        let mut c = base();
        record_preferences(&mut c, &NegotiationPolicy::BEST_QUALITY, NATIVE);
        assert!(has_preference(&c, "color fidelity preferred"));
    }

    #[test]
    fn trace_records_high_refresh_preferred() {
        let mut c = base();
        record_preferences(&mut c, &NegotiationPolicy::BEST_PERFORMANCE, NATIVE);
        assert!(has_preference(&c, "high refresh rate preferred"));
    }

    #[test]
    fn trace_records_power_saving_ordering() {
        let mut c = base();
        record_preferences(&mut c, &NegotiationPolicy::POWER_SAVING, NATIVE);
        assert!(has_preference(&c, "power saving ordering applied"));
    }

    #[test]
    fn trace_records_progressive_mode() {
        let mut c = base(); // interlaced=false
        record_preferences(&mut c, &NegotiationPolicy::BEST_QUALITY, NATIVE);
        assert!(has_preference(&c, "progressive mode preferred"));
    }

    #[test]
    fn trace_no_progressive_step_for_interlaced() {
        let mut c = NegotiatedConfig {
            mode: VideoMode::new(1920, 1080, 60, true),
            ..base()
        };
        record_preferences(&mut c, &NegotiationPolicy::BEST_QUALITY, NATIVE);
        assert!(!has_preference(&c, "progressive mode preferred"));
    }

    #[test]
    fn equal_configs_returns_equal() {
        let a = base();
        let b = base();
        let policy = NegotiationPolicy::BEST_QUALITY;

        assert_eq!(compare_configs(&a, &b, &policy, NATIVE), Ordering::Equal);
    }
}
