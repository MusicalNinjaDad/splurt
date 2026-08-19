//! Test parsing TOC for "Definitely Maybe"
//! https://musicbrainz.org/release/9822581d-98bf-3f97-a94c-4b1350d090aa

use std::path::PathBuf;

use cdtoc::Toc;
use redbook::{
    Frame,
    hex::{hex_to_bytes, parse_cdrom_toc, parse_toc},
};
use windows_sys::Win32::Devices::Cdrom::CDROM_TOC;

/// Load a hex file and parse it as raw bytes
fn load_hex_file(path: &PathBuf) -> Vec<u8> {
    let content = std::fs::read_to_string(path).expect("Failed to read file");
    hex_to_bytes(&content).unwrap()
}

fn toc_from_cdrom_toc(cdrom_toc: CDROM_TOC) -> Toc {
    let audio = cdrom_toc
        .TrackData
        .iter()
        .filter(|track| (1..0xA0).contains(&track.TrackNumber))
        .map(Frame::from)
        .map(|frame| frame.as_usize() as u32)
        .collect();
    let leadout = cdrom_toc
        .TrackData
        .iter()
        .find(|track| track.TrackNumber == 170)
        .map(Frame::from)
        .map(|frame| frame.as_usize() as u32)
        .expect("leadout");
    Toc::from_parts(audio, None, leadout).unwrap()
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

#[test]
fn compare() {
    let path = PathBuf::from("tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa");
    let cdrom_toc_dump = load_hex_file(&path.join("CDROM_TOC.hex"));
    let cdrom_toc = unsafe { parse_cdrom_toc(cdrom_toc_dump) };
    let toc_dump = load_hex_file(&path.join("TOC.hex"));
    let toc_string = parse_toc(toc_dump);
    let toc = Toc::from_cdtoc(toc_string).unwrap();
    assert_eq!(toc, toc_from_cdrom_toc(cdrom_toc))
}
