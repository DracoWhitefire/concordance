//! Source (GPU / transmitter) capability input types.

use display_types::cea861::HdmiForumFrl;

bitflags::bitflags! {
    /// Vendor-specific quirk flags for the source.
    ///
    /// Used to override or relax constraint checks where real hardware diverges
    /// from the specification.
    #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct QuirkFlags: u32 {
        // Reserved for platform-specific flags.
    }
}

/// Display Stream Compression capabilities of the source.
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DscCapabilities {
    /// Supports VESA DSC 1.2a.
    pub dsc_1p2: bool,
    /// Maximum number of slices the source encoder can produce.
    pub max_slices: u8,
    /// Maximum bits per pixel (in 1/16 increments) the source can encode.
    pub max_bpp_x16: u16,
}

/// Capabilities of the source (GPU or other HDMI transmitter).
///
/// The caller fills this struct manually. Populating it from actual GPU hardware
/// is the concern of the source capability discovery library in the integration
/// layer, not this library.
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SourceCapabilities {
    /// Maximum TMDS clock rate the source can output, in kHz.
    pub max_tmds_clock: u32,

    /// Maximum FRL rate the source supports.
    ///
    /// FRL rates are cumulative — this implies support for all lower tiers.
    /// [`HdmiForumFrl::NotSupported`] indicates a TMDS-only source.
    pub max_frl_rate: HdmiForumFrl,

    /// Source Display Stream Compression capabilities, if supported.
    pub dsc: Option<DscCapabilities>,

    /// Vendor-specific quirk overrides.
    pub quirks: QuirkFlags,
}

impl Default for SourceCapabilities {
    fn default() -> Self {
        Self {
            max_tmds_clock: 0,
            max_frl_rate: HdmiForumFrl::NotSupported,
            dsc: None,
            quirks: QuirkFlags::empty(),
        }
    }
}
