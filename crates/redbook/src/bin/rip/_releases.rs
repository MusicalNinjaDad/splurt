//! TODO: FIX BUG - returns wrong index(?)
use redbook::{Disc, musicbrainz::Release};
use std::collections::BTreeMap;
use tabular::{Row, Table, row};

/// Generates a menu to select the correct release, where multiple options are available.
/// Returns `None` if no menu is possible (no musicbrainz data), or required (only one release)
pub fn release_menu(disc: Disc) -> Option<String> {
    let musicbrainz = disc.musicbrainz()?;
    let releases = musicbrainz.releases.as_ref()?;

    if releases.len() <= 1 {
        return None;
    }

    // Group releases by title
    let mut groups: BTreeMap<&str, Vec<&Release>> = BTreeMap::new();
    for release in releases {
        groups.entry(&release.title).or_default().push(release);
    }

    let mut releases = Table::new("{:<}  {:<}  {:<}  {:<}  {:<}");
    releases.add_heading("Multiple releases found. Please select one:");

    // Common across all groups - so can't enumerate releases
    let mut index = 1;

    for (group_title, group_releases) in &mut groups {
        releases.add_heading("");
        releases.add_heading(*group_title);

        let underline = "=".repeat(group_title.len());
        releases.add_heading(underline);

        releases.add_row(Row::from_cells(["", "Date", "Country", "Barcode", ""]));

        group_releases.sort_by_key(|release| {
            release
                .date
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default()
        });
        group_releases.reverse();

        for release in group_releases {
            let date = release
                .date
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            let country = release.country.clone().unwrap_or_default();
            let barcode = release.barcode.clone().unwrap_or_default();
            let disambiguation = release
                .disambiguation
                .as_ref()
                .map(|disambig| {
                    if disambig.is_empty() {
                        String::default()
                    } else {
                        format!("({disambig})")
                    }
                })
                .unwrap_or_default();
            releases.add_row(row!(
                format!("{index}."),
                date,
                country,
                barcode,
                disambiguation
            ));
            index += 1;
        }
    }
    releases.add_heading("");
    Some(releases.to_string())
}

#[cfg(all(test, feature = "test_fixtures"))]
mod tests {
    use redbook::{Track, test_fixtures::albums::TestAlbum};

    use super::*;

    #[test]
    fn the_wall_2() {
        let album = TestAlbum::TheWallDisc2;
        let toc: cdtoc::Toc = album.expected_toc();
        let tracks: Vec<Track> = album.expected_tracks_minimal();
        let leadout = album.expected_leadout();
        let mut disc = Disc::new(toc, tracks, leadout).unwrap();
        let musicbrainz: redbook::musicbrainz::Discid = album.expected_musicbrainz();
        disc.set_musicbrainz(musicbrainz);
        let release_selection = release_menu(disc);
        println!("{}", release_selection.as_ref().unwrap());
        let expected_menu = album.expected_release_menu();
        assert_eq!(release_selection, expected_menu);
    }
}
