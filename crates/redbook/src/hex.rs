//! Hex parsing utilities for CD TOC data

use crate::{Frame, Msf, TocEntry};
use windows_sys::Win32::Devices::Cdrom::CDROM_TOC;

impl TocEntry {
    pub fn from_raw_toc_bytes(data: &[u8]) -> Self {
        let track = data[3];
        let start = Msf::new(data[8] as i8, data[9] as i8, data[10] as i8);
        Self { track, start }
    }
}

/// Convert hex in form `00 01 02` to bytes
pub fn hex_to_bytes(hex: &str) -> Vec<u8> {
    // Remove all non-hex characters (spaces, newlines, etc.)
    let hex_str: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();

    // Convert hex string to bytes
    (0..hex_str.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).expect("Invalid hex digit"))
        .collect()
}

/// Parse raw bytes as a CDROM_TOC structure
///
/// # Safety
/// The caller must ensure `bytes` is exactly the size of CDROM_TOC and properly aligned.
#[allow(unsafe_code)]
pub unsafe fn parse_cdrom_toc(bytes: Vec<u8>) -> CDROM_TOC {
    #[allow(unsafe_code)]
    unsafe { *(bytes.as_ptr() as *const _) }
}

/// Converts a hex dump of raw TOC data to the format
/// `[audio trackcount]+[first audio track address]+[second audio track address]`
/// as used by [cdtoc::Toc::from_cdtoc] and described at
/// https://forum.dbpoweramp.com/forum/other-topics/developers-corner/16082-flac-ogg-vorbis-storage-of-cdtoc?16705-FLAC-amp-Ogg-Vorbis-Storage-of-CDTOC=&s=3ca0c65ee58fc45489103bb1c39bfac0&viewfull=1#post76686
pub fn parse_toc(bytes: Vec<u8>) -> String {
    let mut entries: Vec<_> = bytes
        .chunks_exact(11)
        .map(TocEntry::from_raw_toc_bytes)
        .filter(|entry| entry.track != 0xA0 && entry.track != 0xA1)
        .collect();
    entries.sort_by_key(|entry| entry.track);
    let tracks = entries.len() - 1; // Special entry A2 (leadout)
    let timings = entries
        .iter()
        .map(|entry| {
            format!(
                "{frames:02x}+",
                frames = Frame::from(entry.start).as_usize()
            )
        })
        .collect::<String>();
    format!("{tracks:02x}+{timings}")
        .trim_end_matches("+")
        .to_string()
}
