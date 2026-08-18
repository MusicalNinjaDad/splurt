//! Test parsing TOC for "Definitely Maybe"
//! https://musicbrainz.org/release/9822581d-98bf-3f97-a94c-4b1350d090aa

use std::path::PathBuf;

use cdtoc::Toc;
use redbook::hex::{hex_to_bytes, parse_cdrom_toc, parse_toc};

/// Load a hex file and parse it as raw bytes
fn load_hex_file(path: &PathBuf) -> Vec<u8> {
    let content = std::fs::read_to_string(path).expect("Failed to read file");
    hex_to_bytes(&content)
}

#[test]
fn cdrom_toc() {
    let path = PathBuf::from("tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/CDROM_TOC.hex");
    let toc_dump = load_hex_file(&path);
    let toc = unsafe { parse_cdrom_toc(toc_dump) };
    assert_eq!(toc.FirstTrack, 1);
    assert_eq!(toc.LastTrack, 11);
    assert_eq!(toc.TrackData[0].Address, [0, 0, 0, 33])
}

#[test]
fn load_toc() {
    let path = PathBuf::from("tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/TOC.hex");
    let toc_dump = load_hex_file(&path);
    let toc_string = parse_toc(toc_dump);
    dbg!(&toc_string);
    let toc = Toc::from_cdtoc(toc_string).unwrap();
    assert_eq!(toc.audio_len(), 11)
}
