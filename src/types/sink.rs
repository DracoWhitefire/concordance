//! Sink (display) capability input type.

use display_types::ColorCapabilities;
use display_types::cea861::{ColorimetryBlock, HdmiForumSinkCap, HdmiVsdb, HdrStaticMetadata};

#[cfg(any(feature = "alloc", feature = "std"))]
use alloc::vec::Vec;
#[cfg(any(feature = "alloc", feature = "std"))]
use display_types::VideoMode;

/// A sorted, deduplicated list of video modes.
///
/// Constructed via [`SupportedModes::from_vec`], which normalises the input on
/// entry so that every downstream consumer — including the enumerator — can rely
/// on the invariant unconditionally.
///
/// Available in `alloc` and `std` tiers only.
#[cfg(any(feature = "alloc", feature = "std"))]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SupportedModes(Vec<VideoMode>);

#[cfg(any(feature = "alloc", feature = "std"))]
impl SupportedModes {
    /// Deduplicates `modes`, returning the normalised list and any duplicate
    /// entries that were removed. Insertion order of the first occurrence of
    /// each mode is preserved.
    pub fn from_vec(modes: Vec<VideoMode>) -> (Self, Vec<VideoMode>) {
        let mut seen: Vec<VideoMode> = Vec::with_capacity(modes.len());
        let mut duplicates = Vec::new();
        for mode in modes {
            // Linear scan is O(n²), but EDID mode lists are small (< 100 entries)
            // and `HashSet` is not available in `alloc`-only builds.
            if seen.contains(&mode) {
                duplicates.push(mode);
            } else {
                seen.push(mode);
            }
        }
        (SupportedModes(seen), duplicates)
    }

    /// Returns the modes as a slice.
    pub fn as_slice(&self) -> &[VideoMode] {
        &self.0
    }
}

/// Warning produced by [`sink_capabilities_from_display`] during EDID parsing.
///
/// Returned alongside [`SinkCapabilities`] to surface construction-time anomalies
/// without making them fatal. The list is empty for well-formed EDIDs.
#[cfg(any(feature = "alloc", feature = "std"))]
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum SinkBuildWarning {
    /// One or more video modes appeared more than once in the EDID and were removed.
    #[error("{} duplicate video mode(s) removed", .0.len())]
    DuplicateModes(Vec<VideoMode>),
}

/// Capabilities of the connected display.
///
/// The caller fills this struct manually, or constructs it from a parsed
/// [`DisplayCapabilities`][display_types::DisplayCapabilities] via
/// [`sink_capabilities_from_display`].
#[non_exhaustive]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct SinkCapabilities {
    /// Video modes declared by the display.
    ///
    /// Absent in bare `no_std` builds; [`is_config_viable`][crate::is_config_viable]
    /// validates a caller-supplied candidate rather than enumerating one.
    #[cfg(any(feature = "alloc", feature = "std"))]
    pub supported_modes: SupportedModes,

    /// Maximum pixel clock in MHz (from EDID range limits descriptor).
    pub max_pixel_clock_mhz: Option<u16>,

    /// Minimum vertical rate in Hz.
    pub min_v_rate: Option<u16>,

    /// Maximum vertical rate in Hz.
    pub max_v_rate: Option<u16>,

    /// Supported color format and bit-depth combinations.
    ///
    /// Derived from the EDID base block color encoding field, the Deep Color flags
    /// in the HDMI VSDB and HF-SCDB, and the CEA/CTA extension YCbCr 4:2:0 blocks.
    /// The integration layer is responsible for assembling this from those sources.
    pub color_capabilities: ColorCapabilities,

    /// HDMI 1.x capabilities (from HDMI VSDB; `None` if not present).
    pub hdmi_vsdb: Option<HdmiVsdb>,

    /// HDMI 2.1 capabilities (from HF-SCDB; `None` for pre-HDMI-2.1 sinks).
    pub hdmi_forum: Option<HdmiForumSinkCap>,

    /// HDR static metadata capabilities.
    pub hdr_static: Option<HdrStaticMetadata>,

    /// Colorimetry standards supported.
    pub colorimetry: Option<ColorimetryBlock>,
}

/// Derives [`SinkCapabilities`] from a parsed [`DisplayCapabilities`][display_types::DisplayCapabilities].
///
/// Extracts all fields that can be determined from the parsed EDID data, including
/// the CEA-861 extension block (stored at extension tag `0x02`). Fields that cannot
/// be derived from EDID data — specifically [`SinkCapabilities::hdmi_vsdb`]'s quirk
/// overrides — default to their zero values and can be set after the call.
///
/// The HDMI Forum Sink Capability Data Block is preferred over the HDMI Forum
/// Vendor-Specific Data Block when both are present (the former is the HDMI 2.1
/// mechanism; the latter is the older HDMI 2.0 mechanism for the same data).
#[cfg(any(feature = "alloc", feature = "std"))]
pub fn sink_capabilities_from_display(
    caps: &display_types::DisplayCapabilities,
) -> (SinkCapabilities, Vec<SinkBuildWarning>) {
    use display_types::cea861::Cea861Capabilities;
    use display_types::{ColorBitDepth, color_capabilities_from_edid};

    let cea = caps.get_extension_data::<Cea861Capabilities>(0x02);

    let hdmi_vsdb = cea.and_then(|c| c.hdmi_vsdb.as_ref());
    // Prefer HF-SCDB (HDMI 2.1) over HF-VSDB (HDMI 2.0); both carry the same structure.
    let hdmi_forum = cea.and_then(|c| c.hf_scdb.as_ref().or(c.hf_vsdb.as_ref()));

    let mut color_capabilities = color_capabilities_from_edid(
        caps.digital_color_encoding,
        caps.color_bit_depth,
        hdmi_vsdb,
        hdmi_forum,
    );

    // If the YCbCr 4:2:0 Video Data Block or Capability Map Data Block is present,
    // at least one mode supports YCbCr 4:2:0 at 8 bpc — add baseline support.
    let has_y420_vdb =
        cea.is_some_and(|c| !c.y420_vics.is_empty() || !c.y420_capability_map.is_empty());
    if has_y420_vdb {
        color_capabilities.ycbcr420 = color_capabilities.ycbcr420.with(ColorBitDepth::Depth8);
    }

    let (supported_modes, duplicates) = SupportedModes::from_vec(caps.supported_modes.clone());
    let mut warnings = Vec::new();
    if !duplicates.is_empty() {
        warnings.push(SinkBuildWarning::DuplicateModes(duplicates));
    }

    (
        SinkCapabilities {
            supported_modes,
            max_pixel_clock_mhz: caps.max_pixel_clock_mhz,
            min_v_rate: caps.min_v_rate,
            max_v_rate: caps.max_v_rate,
            color_capabilities,
            hdmi_vsdb: cea.and_then(|c| c.hdmi_vsdb.clone()),
            hdmi_forum: cea.and_then(|c| c.hf_scdb.clone().or_else(|| c.hf_vsdb.clone())),
            hdr_static: cea.and_then(|c| c.hdr_static_metadata.clone()),
            colorimetry: cea.and_then(|c| c.colorimetry),
        },
        warnings,
    )
}
