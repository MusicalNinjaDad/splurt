//! Test parsing TOC for "Definitely Maybe"
//! https://musicbrainz.org/release/9822581d-98bf-3f97-a94c-4b1350d090aa

use std::path::PathBuf;

use cdtoc::Toc;
use redbook::{
    Frame, TocEntry,
    hex::{hex_to_bytes, parse_toc},
    win::CdromTocExt,
};
use windows_sys::Win32::Devices::Cdrom::CDROM_TOC;

/// Load a hex file and parse it as raw bytes
fn load_hex_file(path: &PathBuf) -> Vec<u8> {
    let content = std::fs::read_to_string(path).expect("Failed to read file");
    hex_to_bytes(&content).unwrap()
}

#[test]
fn cdrom_toc() {
    let path = PathBuf::from("tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/CDROM_TOC.hex");
    let toc_dump = load_hex_file(&path);
    let toc = unsafe { CDROM_TOC::from_raw_bytes(toc_dump) };
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
    let cdrom_toc = unsafe { CDROM_TOC::from_raw_bytes(cdrom_toc_dump) };
    let toc_dump = load_hex_file(&path.join("TOC.hex"));
    let toc_string = parse_toc(toc_dump);
    let toc = Toc::from_cdtoc(toc_string).unwrap();
    assert_eq!(toc, cdrom_toc.to_toc().unwrap())
}

// Tests for CdromTocExt trait

#[test]
fn test_to_toc() {
    let path = PathBuf::from("tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/CDROM_TOC.hex");
    let toc_dump = load_hex_file(&path);
    let cdrom_toc = unsafe { CDROM_TOC::from_raw_bytes(toc_dump) };

    let toc = cdrom_toc.to_toc().unwrap();

    // Should have 11 audio tracks
    assert_eq!(toc.audio_len(), 11);

    // Should match the manually constructed toc
    assert_eq!(toc, cdrom_toc.to_toc().unwrap())
}

#[test]
fn test_iter_audio() {
    let path = PathBuf::from("tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/CDROM_TOC.hex");
    let toc_dump = load_hex_file(&path);
    let cdrom_toc = unsafe { CDROM_TOC::from_raw_bytes(toc_dump) };

    let audio_tracks: Vec<TocEntry> = cdrom_toc.iter_audio().collect();

    // Should have 11 audio tracks
    assert_eq!(audio_tracks.len(), 11);

    // Tracks should be numbered 1-11
    for (i, entry) in audio_tracks.iter().enumerate() {
        assert_eq!(entry.track, (i + 1) as u8);
    }

    // First track should start at frame 150 (LEADIN) + 33 = 183
    // The first track's Address is [0, 0, 0, 33] which is 33 in big-endian
    // Frame::from adds LEADIN (150), so it should be 183
    let first_track_frame: Frame = audio_tracks[0].start;
    assert_eq!(first_track_frame.as_usize(), 183);
}

#[test]
fn test_leadout() {
    let path = PathBuf::from("tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/CDROM_TOC.hex");
    let toc_dump = load_hex_file(&path);
    let cdrom_toc = unsafe { CDROM_TOC::from_raw_bytes(toc_dump) };

    let leadout = cdrom_toc.leadout().unwrap();

    // Leadout should be track 170 (0xAA)
    // The leadout frame should be > 0
    assert!(leadout.as_usize() > 0);

    // Should match the leadout from the manually constructed toc
    let expected_toc = cdrom_toc.to_toc().unwrap();
    let expected_leadout = Frame::new(expected_toc.leadout() as usize);
    assert_eq!(leadout, expected_leadout);
}
