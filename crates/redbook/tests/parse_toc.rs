//! Test parsing TOC for "Definitely Maybe"
//! https://musicbrainz.org/release/9822581d-98bf-3f97-a94c-4b1350d090aa

use std::path::PathBuf;

use cdtoc::Toc;
use redbook::{
    hex::{hex_to_bytes, parse_toc},
    win::CdromTocExt,
};
use windows_sys::Win32::Devices::Cdrom::CDROM_TOC;

/// Load a hex file and parse it as raw bytes
fn load_hex_file(path: &PathBuf) -> Vec<u8> {
    let content = std::fs::read_to_string(path).unwrap();
    hex_to_bytes(&content).unwrap()
}

#[test]
fn compare_definitely_maybe() {
    let assets = PathBuf::from("tests/assets/definitely_maybe");

    let cdrom_toc_dump = load_hex_file(&assets.join("CDROM_TOC.hex"));
    let cdrom_toc = unsafe { CDROM_TOC::from_raw_bytes(cdrom_toc_dump) };

    let toc_dump = load_hex_file(&assets.join("TOC.hex"));
    let toc_string = parse_toc(toc_dump);
    let toc = Toc::from_cdtoc(toc_string).unwrap();

    assert_eq!(toc, cdrom_toc.to_toc().unwrap())
}
