//! Negotiation policy type and named presets.

/// Controls how the ranker orders validated configurations.
///
/// `NegotiationPolicy` is a const-constructible struct. Named presets are provided
/// for common cases; custom implementations can encode platform-specific priorities.
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NegotiationPolicy {
    /// Prefer native resolution over non-native resolutions.
    pub prefer_native_resolution: bool,

    /// Prefer higher color fidelity (bit depth, color encoding) over higher refresh rate.
    pub prefer_color_fidelity: bool,

    /// Prefer higher refresh rate over other factors after color fidelity.
    pub prefer_high_refresh: bool,

    /// Penalize configurations that require DSC.
    pub penalize_dsc: bool,
}

impl NegotiationPolicy {
    /// Prefer maximum quality: native resolution, max color fidelity, then refresh rate.
    pub const BEST_QUALITY: Self = Self {
        prefer_native_resolution: true,
        prefer_color_fidelity: true,
        prefer_high_refresh: false,
        penalize_dsc: true,
    };

    /// Prefer maximum performance: native resolution, max refresh rate, then color fidelity.
    pub const BEST_PERFORMANCE: Self = Self {
        prefer_native_resolution: true,
        prefer_color_fidelity: false,
        prefer_high_refresh: true,
        penalize_dsc: false,
    };

    /// Prefer power saving: lower refresh rates and simpler configurations are ranked higher.
    pub const POWER_SAVING: Self = Self {
        prefer_native_resolution: true,
        prefer_color_fidelity: false,
        prefer_high_refresh: false,
        penalize_dsc: true,
    };
}

impl Default for NegotiationPolicy {
    fn default() -> Self {
        Self::BEST_QUALITY
    }
}
