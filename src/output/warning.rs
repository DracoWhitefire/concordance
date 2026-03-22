//! Built-in warning and violation types.

/// Non-fatal warning attached to an accepted configuration.
///
/// Warnings do not prevent a mode from being offered; they give the caller enough
/// information to surface concerns to the user or log them. Custom constraint
/// engines and rankers can define their own warning types by specifying an
/// associated `Warning` type bounded by [`Diagnostic`][crate::Diagnostic].
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, thiserror::Error)]
pub enum Warning {
    /// DSC is required for this configuration; lossy compression is active.
    #[error("DSC required; lossy compression active")]
    DscActive,

    /// Cable bandwidth is marginal for the selected mode and may degrade under load.
    #[error("cable bandwidth marginal for selected mode")]
    CableBandwidthMarginal,
}

/// A constraint violation produced when a candidate configuration is rejected.
///
/// Violations are returned by [`is_config_viable`][crate::is_config_viable] and by
/// the constraint engine during pipeline runs. Custom constraint engines can define
/// their own violation types via the `ConstraintEngine::Violation` associated type.
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, thiserror::Error)]
pub enum Violation {
    /// Required pixel clock exceeds what the sink, source, or cable supports.
    #[error("pixel clock {required_mhz} MHz exceeds limit of {limit_mhz} MHz")]
    PixelClockExceeded {
        /// Required pixel clock in MHz.
        required_mhz: u32,
        /// Binding limit in MHz.
        limit_mhz: u32,
    },

    /// Required FRL rate exceeds what the sink, source, or cable supports.
    #[error("required FRL rate exceeds supported maximum")]
    FrlRateExceeded,

    /// The selected color encoding is not supported by the sink.
    #[error("color encoding not supported by sink")]
    ColorEncodingUnsupported,

    /// The selected bit depth is not supported by the sink.
    #[error("bit depth not supported by sink")]
    BitDepthUnsupported,

    /// DSC is required but not supported by all parties.
    #[error("DSC required but not supported")]
    DscUnsupported,

    /// The vertical refresh rate is outside the sink's declared range.
    #[error("refresh rate {rate_hz} Hz outside sink range [{min_hz}, {max_hz}] Hz")]
    RefreshRateOutOfRange {
        /// Refresh rate of the candidate in Hz.
        rate_hz: u16,
        /// Minimum declared by the sink.
        min_hz: u16,
        /// Maximum declared by the sink.
        max_hz: u16,
    },
}
