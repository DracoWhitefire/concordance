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

#[cfg(all(test, any(feature = "alloc", feature = "std")))]
mod tests {
    use super::*;
    use display_types::DisplayCapabilities;
    use display_types::cea861::{
        Cea861Capabilities, Cea861Flags, HdmiForumFrl, HdmiForumSinkCap, HdmiVsdb, HdmiVsdbFlags,
    };
    use display_types::{ColorBitDepth, VideoMode};

    // ── helpers ───────────────────────────────────────────────────────────────

    /// Minimal `HdmiForumSinkCap` identified by its `max_tmds_rate_mhz` for easy
    /// equality checks in tests.
    fn hf_sink(max_tmds_rate_mhz: u16) -> HdmiForumSinkCap {
        HdmiForumSinkCap::new(
            1,
            max_tmds_rate_mhz,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            HdmiForumFrl::NotSupported,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            false,
            None,
            None,
            None,
        )
    }

    /// Minimal `HdmiVsdb`.
    fn vsdb() -> HdmiVsdb {
        HdmiVsdb::new(0, HdmiVsdbFlags::empty(), None, None, None, None, None)
    }

    /// Attaches a `Cea861Capabilities` to a `DisplayCapabilities` at tag `0x02`.
    fn with_cea(caps: &mut DisplayCapabilities, cea: Cea861Capabilities) {
        caps.set_extension_data(0x02, cea);
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    /// With no CEA extension block attached, the function returns defaults with no warnings.
    #[test]
    fn no_cea_extension_yields_defaults() {
        let caps = DisplayCapabilities::default();
        let (sink, warnings) = sink_capabilities_from_display(&caps);
        assert!(warnings.is_empty());
        assert!(sink.hdmi_forum.is_none());
        assert!(sink.hdmi_vsdb.is_none());
        assert!(sink.hdr_static.is_none());
        assert!(sink.colorimetry.is_none());
    }

    /// Scalar range-limit fields are copied directly from the input.
    #[test]
    fn scalar_fields_are_copied() {
        let mut caps = DisplayCapabilities::default();
        caps.max_pixel_clock_mhz = Some(300);
        caps.min_v_rate = Some(24);
        caps.max_v_rate = Some(144);
        let (sink, _) = sink_capabilities_from_display(&caps);
        assert_eq!(sink.max_pixel_clock_mhz, Some(300));
        assert_eq!(sink.min_v_rate, Some(24));
        assert_eq!(sink.max_v_rate, Some(144));
    }

    /// Unique modes are accepted without any warning.
    #[test]
    fn unique_modes_produce_no_warning() {
        let mut caps = DisplayCapabilities::default();
        caps.supported_modes = alloc::vec![
            VideoMode::new(1920, 1080, 60, false),
            VideoMode::new(3840, 2160, 60, false),
        ];
        let (sink, warnings) = sink_capabilities_from_display(&caps);
        assert!(warnings.is_empty());
        assert_eq!(sink.supported_modes.as_slice().len(), 2);
    }

    /// A duplicate mode in the input triggers a `DuplicateModes` warning and the
    /// deduplicated list contains only the first occurrence.
    #[test]
    fn duplicate_modes_produce_warning() {
        let mode = VideoMode::new(1920, 1080, 60, false);
        let mut caps = DisplayCapabilities::default();
        caps.supported_modes = alloc::vec![mode.clone(), mode.clone()];
        let (sink, warnings) = sink_capabilities_from_display(&caps);
        assert_eq!(sink.supported_modes.as_slice().len(), 1);
        assert!(
            warnings
                .iter()
                .any(|w| matches!(w, SinkBuildWarning::DuplicateModes(_))),
            "expected a DuplicateModes warning"
        );
    }

    /// When the CEA block carries an HF-SCDB, it is used as `hdmi_forum`.
    #[test]
    fn cea_hf_scdb_populates_hdmi_forum() {
        let mut caps = DisplayCapabilities::default();
        let mut cea = Cea861Capabilities::new(Cea861Flags::empty());
        cea.hf_scdb = Some(hf_sink(600));
        with_cea(&mut caps, cea);
        let (sink, _) = sink_capabilities_from_display(&caps);
        assert!(sink.hdmi_forum.is_some());
        assert_eq!(sink.hdmi_forum.as_ref().unwrap().max_tmds_rate_mhz, 600);
    }

    /// When only HF-VSDB is present (no HF-SCDB), it is used as `hdmi_forum`.
    #[test]
    fn cea_hf_vsdb_used_when_scdb_absent() {
        let mut caps = DisplayCapabilities::default();
        let mut cea = Cea861Capabilities::new(Cea861Flags::empty());
        cea.hf_vsdb = Some(hf_sink(340));
        with_cea(&mut caps, cea);
        let (sink, _) = sink_capabilities_from_display(&caps);
        assert!(sink.hdmi_forum.is_some());
        assert_eq!(sink.hdmi_forum.as_ref().unwrap().max_tmds_rate_mhz, 340);
    }

    /// When both HF-SCDB and HF-VSDB are present, HF-SCDB (HDMI 2.1) takes precedence.
    #[test]
    fn hf_scdb_preferred_over_hf_vsdb() {
        let mut caps = DisplayCapabilities::default();
        let mut cea = Cea861Capabilities::new(Cea861Flags::empty());
        cea.hf_scdb = Some(hf_sink(600)); // HDMI 2.1 — should win
        cea.hf_vsdb = Some(hf_sink(340)); // HDMI 2.0
        with_cea(&mut caps, cea);
        let (sink, _) = sink_capabilities_from_display(&caps);
        assert_eq!(
            sink.hdmi_forum.as_ref().unwrap().max_tmds_rate_mhz,
            600,
            "HF-SCDB must be preferred over HF-VSDB"
        );
    }

    /// An HDMI 1.x VSDB in the CEA block is copied to `hdmi_vsdb`.
    #[test]
    fn cea_hdmi_vsdb_is_propagated() {
        let mut caps = DisplayCapabilities::default();
        let mut cea = Cea861Capabilities::new(Cea861Flags::empty());
        cea.hdmi_vsdb = Some(vsdb());
        with_cea(&mut caps, cea);
        let (sink, _) = sink_capabilities_from_display(&caps);
        assert!(sink.hdmi_vsdb.is_some());
    }

    /// A non-empty `y420_vics` list causes YCbCr 4:2:0 at 8 bpc to be added to
    /// the color capabilities, even when no HF-SCDB deep-color flags are set.
    #[test]
    fn y420_vics_adds_baseline_ycbcr420() {
        let mut caps = DisplayCapabilities::default();
        let mut cea = Cea861Capabilities::new(Cea861Flags::empty());
        cea.y420_vics = alloc::vec![96]; // 4K@60 VIC
        with_cea(&mut caps, cea);
        let (sink, _) = sink_capabilities_from_display(&caps);
        assert!(
            sink.color_capabilities
                .ycbcr420
                .supports(ColorBitDepth::Depth8),
            "y420_vics must add YCbCr 4:2:0 8 bpc baseline"
        );
    }

    /// A non-empty `y420_capability_map` also triggers the baseline YCbCr 4:2:0 addition.
    #[test]
    fn y420_capability_map_adds_baseline_ycbcr420() {
        let mut caps = DisplayCapabilities::default();
        let mut cea = Cea861Capabilities::new(Cea861Flags::empty());
        cea.y420_capability_map = alloc::vec![0xFF];
        with_cea(&mut caps, cea);
        let (sink, _) = sink_capabilities_from_display(&caps);
        assert!(
            sink.color_capabilities
                .ycbcr420
                .supports(ColorBitDepth::Depth8),
            "y420_capability_map must add YCbCr 4:2:0 8 bpc baseline"
        );
    }

    /// HDR static metadata in the CEA block is propagated to `hdr_static`.
    #[test]
    fn hdr_static_metadata_is_propagated() {
        use display_types::cea861::{HdrEotf, HdrStaticMetadata};
        let mut caps = DisplayCapabilities::default();
        let mut cea = Cea861Capabilities::new(Cea861Flags::empty());
        cea.hdr_static_metadata = Some(HdrStaticMetadata::new(
            HdrEotf::empty(),
            0,
            None,
            None,
            None,
        ));
        with_cea(&mut caps, cea);
        let (sink, _) = sink_capabilities_from_display(&caps);
        assert!(sink.hdr_static.is_some());
    }

    /// Colorimetry data in the CEA block is propagated to `colorimetry`.
    #[test]
    fn colorimetry_is_propagated() {
        use display_types::cea861::{ColorimetryBlock, ColorimetryFlags};
        let mut caps = DisplayCapabilities::default();
        let mut cea = Cea861Capabilities::new(Cea861Flags::empty());
        cea.colorimetry = Some(ColorimetryBlock::new(ColorimetryFlags::empty(), 0));
        with_cea(&mut caps, cea);
        let (sink, _) = sink_capabilities_from_display(&caps);
        assert!(sink.colorimetry.is_some());
    }

    /// `color_capabilities` reflects the VSDB deep-color flags when a VSDB is present.
    #[test]
    fn vsdb_deep_color_flags_populate_color_capabilities() {
        let mut caps = DisplayCapabilities::default();
        caps.digital_color_encoding = Some(display_types::DigitalColorEncoding::Rgb444);
        let mut cea = Cea861Capabilities::new(Cea861Flags::empty());
        cea.hdmi_vsdb = Some(HdmiVsdb::new(
            0,
            HdmiVsdbFlags::DC_30BIT | HdmiVsdbFlags::DC_36BIT,
            None,
            None,
            None,
            None,
            None,
        ));
        with_cea(&mut caps, cea);
        let (sink, _) = sink_capabilities_from_display(&caps);
        assert!(
            sink.color_capabilities
                .rgb444
                .supports(ColorBitDepth::Depth10)
        );
        assert!(
            sink.color_capabilities
                .rgb444
                .supports(ColorBitDepth::Depth12)
        );
        assert!(
            !sink
                .color_capabilities
                .rgb444
                .supports(ColorBitDepth::Depth16)
        );
    }
}
