//! Built-in warning and violation types.

/// Identifies which party imposed the binding limit in a bandwidth violation.
///
/// When a bandwidth check fails, this value tells the caller *which* end of the
/// link is the bottleneck so they can suggest the right remediation:
/// - [`Sink`][LimitSource::Sink] — the display's declared ceiling is too low.
/// - [`Source`][LimitSource::Source] — the GPU or transmitter cannot drive the required rate.
/// - [`Cable`][LimitSource::Cable] — the cable cannot carry the required bandwidth;
///   replacing it with a higher-rated cable may resolve the violation.
///
/// When multiple parties share the same binding limit, `Cable` takes priority over
/// `Source`, which takes priority over `Sink`, because cable replacement is the most
/// actionable remediation.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitSource {
    /// The sink's declared capability is the binding constraint.
    Sink,
    /// The source's declared capability is the binding constraint.
    Source,
    /// The cable's declared capability is the binding constraint.
    Cable,
}

impl core::fmt::Display for LimitSource {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            LimitSource::Sink => f.write_str("sink"),
            LimitSource::Source => f.write_str("source"),
            LimitSource::Cable => f.write_str("cable"),
        }
    }
}

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
    /// Required pixel clock exceeds the sink's declared maximum.
    #[error("pixel clock {required_mhz} MHz exceeds {limit_source} limit of {limit_mhz} MHz")]
    PixelClockExceeded {
        /// Required pixel clock in MHz.
        required_mhz: u32,
        /// Binding limit in MHz.
        limit_mhz: u32,
        /// Which party imposed the binding limit.
        limit_source: LimitSource,
    },

    /// Required TMDS character rate exceeds what the sink, source, or cable supports.
    #[error("TMDS clock {required_mhz} MHz exceeds {limit_source} limit of {limit_mhz} MHz")]
    TmdsClockExceeded {
        /// Required TMDS character rate in MHz.
        required_mhz: u32,
        /// Binding limit in MHz.
        limit_mhz: u32,
        /// Which party imposed the binding limit.
        limit_source: LimitSource,
    },

    /// Required FRL rate exceeds what the sink, source, or cable supports.
    #[error("FRL rate {requested:?} exceeds {limit_source} limit of {limit:?}")]
    FrlRateExceeded {
        /// The FRL rate requested by the candidate configuration.
        requested: display_types::cea861::HdmiForumFrl,
        /// The effective ceiling imposed by the binding party.
        limit: display_types::cea861::HdmiForumFrl,
        /// Which party imposed the binding limit.
        limit_source: LimitSource,
    },

    /// The selected color encoding is not supported by the sink.
    #[error("color encoding not supported by sink")]
    ColorEncodingUnsupported,

    /// A non-YCbCr 4:2:0 encoding was requested for a mode that only supports YCbCr 4:2:0.
    ///
    /// The mode appears in the sink's Y420 Video Data Block, which declares it as a
    /// YCbCr 4:2:0-only mode per CTA-861-H §7.5.11.
    #[error("mode only supports YCbCr 4:2:0; other encodings are not valid for this mode")]
    EncodingRestrictedToYCbCr420,

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
