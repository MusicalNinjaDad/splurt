//! Test parsing TOC for "Definitely Maybe"
//! https://musicbrainz.org/release/9822581d-98bf-3f97-a94c-4b1350d090aa

use std::path::{Path, PathBuf};

use windows_sys::Win32::Devices::Cdrom::CDROM_TOC;

/// Load a hex file and parse it as raw bytes
fn load_hex_file<P: AsRef<Path>>(path: &P) -> Vec<u8> {
    let content = std::fs::read_to_string(path).expect("Failed to read file");
    // Remove all non-hex characters (spaces, newlines, etc.)
    let hex_str: String = content.chars().filter(|c| c.is_ascii_hexdigit()).collect();

    // Convert hex string to bytes
    (0..hex_str.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).expect("Invalid hex digit"))
        .collect()
}

unsafe fn load_cdrom_toc(bytes: Vec<u8>) -> CDROM_TOC {
    unsafe { *(bytes.as_ptr() as *const _) }
}

#[test]
fn load_toc() {
    let path = PathBuf::from("tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/CDROM_TOC.hex");
    let toc_dump = load_hex_file(&path);
    let toc = unsafe { load_cdrom_toc(toc_dump) };
    assert_eq!(toc.FirstTrack, 1);
    assert_eq!(toc.LastTrack, 11);
}
