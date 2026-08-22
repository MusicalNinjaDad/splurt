use redbook::Disc;

/// Generates a menu to select the correct release, where multiple options are available.
/// Returns `None` if no menu is possible (no musicbrainz data), or required (only one release)
pub fn release_menu(disc: Disc) -> Option<String> {
    todo!("implement based on the example formatting in crates/redbook/tests/assets/the_wall/disc2/release_selection.txt")
}

#[cfg(all(test, feature = "test_fixtures"))]
mod tests {
    use redbook::{test_fixtures::albums::TestAlbum};

use super::*;

    #[test]
    fn the_wall_2() {
        let album = TestAlbum::TheWallDisc2;
        let toc: cdtoc::Toc = album.expected_toc(); // TODO: Does not exist - please create it.
        let tracks: Vec<redbook::Track> = album.expected_tracks(); // TODO: Does not exist - please create it.
        let leadout = album.expected_leadout();
        let mut disc = Disc::new(toc, tracks, leadout).unwrap();
        let musicbrainz: redbook::musicbrainz::Discid = album.expected_musicbrainz(); // TODO: Does not exist - please create it.
        disc.set_musicbrainz(); // TODO: Does not exist - please create it.
        let release_selection = release_menu(disc);
        let expected_menu: Some(String) = album.expected_release_menu(); // TODO: Does not exist - please create it. The contents should be loaded at runtime from assets_path().join("release_selection.txt")
        assert_eq!(release_selection, expected_menu);
    }
}
