//! Test parsing TOC for various albums

use cdtoc::Toc;
use redbook::{hex::parse_toc, test_fixtures::albums::TestAlbum, win::CdromTocExt};
use rstest::rstest;
use windows_sys::Win32::Devices::Cdrom::CDROM_TOC;

#[rstest]
#[case(TestAlbum::DefinitelyMaybe)]
#[case(TestAlbum::TheWallDisc1)]
#[case(TestAlbum::TheWallDisc2)]
fn compare_toc(#[case] album: TestAlbum) {
    let cdrom_toc_dump = redbook::test_fixtures::load_hex_file(&album.cdrom_toc_path());
    let cdrom_toc = unsafe { CDROM_TOC::from_raw_bytes(cdrom_toc_dump) };

    let toc_dump = redbook::test_fixtures::load_hex_file(&album.toc_path());
    let toc_string = parse_toc(toc_dump);
    let toc = Toc::from_cdtoc(toc_string).unwrap();

    assert_eq!(toc, cdrom_toc.to_toc().unwrap())
}
