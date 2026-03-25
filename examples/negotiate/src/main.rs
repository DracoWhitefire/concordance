//! End-to-end HDMI mode negotiation using piaf and concordance.
//!
//! Scans connected displays via `/sys/class/drm`, parses each EDID with piaf,
//! derives sink capabilities, then asks concordance to rank viable configurations
//! against a hardcoded HDMI 2.1 source profile.
//!
//! Run with: `cargo run`

use std::fs;
use std::path::Path;

use concordance::{
    CableCapabilities, NegotiatorBuilder, SinkBuildWarning, SourceCapabilities,
    sink_capabilities_from_display,
};
use display_types::cea861::HdmiForumFrl;
use piaf::{ExtensionLibrary, capabilities_from_edid, parse_edid};

// ---------------------------------------------------------------------------
// Source profile
//
// Represents a typical HDMI 2.1 GPU output. In a real integration this would
// come from a kernel driver query or a hardware capabilities database.
// ---------------------------------------------------------------------------

fn source_profile() -> SourceCapabilities {
    let mut s = SourceCapabilities::default();
    // 600 MHz — the HDMI 2.1 TMDS ceiling.
    s.max_tmds_clock = 600_000;
    // Full 48 Gbps FRL capability.
    s.max_frl_rate = HdmiForumFrl::Rate12Gbps4Lanes;
    s
}

fn main() {
    println!("--- Negotiating HDMI configurations for connected displays ---");

    let drm_path = Path::new("/sys/class/drm");
    if !drm_path.exists() {
        eprintln!("Error: /sys/class/drm not found. This example only works on Linux.");
        return;
    }

    let library = ExtensionLibrary::with_standard_handlers();
    let source = source_profile();
    // No cable information available — assume the cable is not the binding constraint.
    let cable = CableCapabilities::unconstrained();

    let mut found = 0;

    if let Ok(entries) = fs::read_dir(drm_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let edid_path = path.join("edid");
            if !edid_path.exists() {
                continue;
            }

            let Ok(bytes) = fs::read(&edid_path) else {
                continue;
            };
            if bytes.is_empty() || bytes.iter().all(|&b| b == 0) {
                continue;
            }

            println!("\n=== Connector: {} ===", name);

            let parsed = match parse_edid(&bytes, &library) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("  parse error: {e}");
                    continue;
                }
            };

            let display_caps = capabilities_from_edid(&parsed, &library);

            if let Some(name) = display_caps.display_name.as_deref() {
                println!("  Display:      {}", name);
            }
            if let Some(mfr) = display_caps.manufacturer.as_ref() {
                println!("  Manufacturer: {}", mfr.as_str());
            }

            let (sink, warnings) = sink_capabilities_from_display(&display_caps);

            for w in &warnings {
                match w {
                    SinkBuildWarning::DuplicateModes(dups) => {
                        eprintln!("  warning: {} duplicate mode(s) removed", dups.len());
                    }
                    _ => eprintln!("  warning: {w}"),
                }
            }

            println!(
                "  Modes:        {} supported, pixel clock max: {}",
                sink.supported_modes.as_slice().len(),
                sink.max_pixel_clock_mhz
                    .map(|c| format!("{c} MHz"))
                    .unwrap_or_else(|| "unspecified".into()),
            );

            if let Some(forum) = &sink.hdmi_forum {
                println!(
                    "  HDMI 2.1:     max FRL {:?}, max TMDS {} MHz",
                    forum.max_frl_rate, forum.max_tmds_rate_mhz,
                );
            }

            if sink.hdr_static.is_some() {
                println!("  HDR:          static metadata present");
            }

            let configs = NegotiatorBuilder::default().negotiate(&sink, &source, &cable);

            if configs.is_empty() {
                println!("  Negotiated:   no configurations");
            } else {
                println!("  Negotiated:   {} configuration(s)", configs.len());
                println!(
                    "\n  {:<20} {:<6} {:<14} {:<22} {:<5}",
                    "Mode", "Hz", "Color", "Bit depth", "DSC"
                );
                println!("  {}", "-".repeat(72));
                for cfg in &configs {
                    println!(
                        "  {:<20} {:<6} {:<14} {:<22} {:<5}",
                        format!("{}x{}", cfg.mode.width, cfg.mode.height),
                        cfg.mode.refresh_rate,
                        format!("{:?}", cfg.color_encoding),
                        format!("{:?}", cfg.bit_depth),
                        if cfg.dsc_required { "yes" } else { "no" },
                    );
                }
                println!();
            }

            found += 1;
        }
    }

    if found == 0 {
        println!("\nNo connected displays with EDID data found.");
    }
}
