//! Test parsing TOC for various albums

use std::path::PathBuf;

use cdtoc::Toc;
use redbook::{
    hex::{hex_to_bytes, parse_toc},
    win::CdromTocExt,
};
use rstest::rstest;
use windows_sys::Win32::Devices::Cdrom::CDROM_TOC;

/// Load a hex file and parse it as raw bytes
fn load_hex_file(path: &PathBuf) -> Vec<u8> {
    let content = std::fs::read_to_string(path).unwrap();
    hex_to_bytes(&content).unwrap()
}

/// Test album for parameterized tests
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TestAlbum {
    DefinitelyMaybe,
    TheWallDisc1,
    TheWallDisc2,
}

impl TestAlbum {
    fn assets_path(&self) -> PathBuf {
        match self {
            TestAlbum::DefinitelyMaybe => PathBuf::from("tests/assets/definitely_maybe"),
            TestAlbum::TheWallDisc1 => PathBuf::from("tests/assets/the_wall/disc1"),
            TestAlbum::TheWallDisc2 => PathBuf::from("tests/assets/the_wall/disc2"),
        }
    }
}

#[rstest]
#[case(TestAlbum::DefinitelyMaybe)]
#[case(TestAlbum::TheWallDisc1)]
#[case(TestAlbum::TheWallDisc2)]
fn compare_toc(#[case] album: TestAlbum) {
    let assets = album.assets_path();

    let cdrom_toc_dump = load_hex_file(&assets.join("CDROM_TOC.hex"));
    let cdrom_toc = unsafe { CDROM_TOC::from_raw_bytes(cdrom_toc_dump) };

    let toc_dump = load_hex_file(&assets.join("TOC.hex"));
    let toc_string = parse_toc(toc_dump);
    let toc = Toc::from_cdtoc(toc_string).unwrap();

    assert_eq!(toc, cdrom_toc.to_toc().unwrap())
}
