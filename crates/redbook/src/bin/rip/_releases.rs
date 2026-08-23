use redbook::{Disc, musicbrainz::Release};
use std::collections::BTreeMap;
use tabular::{Row, Table, row};

/// Generates a menu to select the correct release, where multiple options are available.
/// Returns `None` if no menu is possible (no musicbrainz data), or required (only one release)
pub fn release_menu(disc: &Disc) -> Option<ReleaseMenu<'_>> {
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

    let mut release_table = Table::new("{:<}  {:<}  {:<}  {:<}  {:<}");
    release_table.add_heading("Multiple releases found. Please select one:");

    // Common across all groups - so can't enumerate releases
    let mut index = 1;
    let mut ordered_releases = Vec::<&Release>::new();

    for (group_title, group_releases) in &mut groups {
        release_table.add_heading("");
        release_table.add_heading(*group_title);

        let underline = "=".repeat(group_title.len());
        release_table.add_heading(underline);

        release_table.add_row(Row::from_cells(["", "Date", "Country", "Barcode", ""]));

        group_releases.sort_by_key(|release| {
            release
                .date
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default()
        });
        group_releases.reverse();
        ordered_releases.extend(group_releases.iter());

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
            release_table.add_row(row!(
                format!("{index}."),
                date,
                country,
                barcode,
                disambiguation
            ));
            index += 1;
        }
    }
    release_table.add_heading("");
    Some(ReleaseMenu {
        table: release_table.to_string(),
        sorted_releases: ordered_releases,
        original_releases: releases.iter().collect(),
    })
}

pub struct ReleaseMenu<'disc> {
    pub table: String,
    pub sorted_releases: Vec<&'disc Release>,
    pub original_releases: Vec<&'disc Release>,
}

impl<'d> ReleaseMenu<'d> {
    pub fn index_for(&self, selection: usize) -> usize {
        let selected = self.sorted_releases.get(selection - 1).unwrap();
        self.original_releases
            .iter()
            .position(|release| release.id == selected.id)
            .unwrap()
    }
}

#[cfg(all(test, feature = "test_fixtures"))]
mod tests {
    use redbook::{Track, test_fixtures::albums::TestAlbum};

    use super::*;

    #[test]
    fn the_wall_2_menu() {
        let album = TestAlbum::TheWallDisc2;
        let toc: cdtoc::Toc = album.expected_toc();
        let tracks: Vec<Track> = album.expected_tracks_minimal();
        let leadout = album.expected_leadout();
        let mut disc = Disc::new(toc, tracks, leadout).unwrap();
        let musicbrainz = album.expected_musicbrainz();
        disc.set_musicbrainz(musicbrainz);
        let ReleaseMenu { table, .. } = release_menu(&disc).unwrap();
        println!("{}", table);
        let expected_menu = album.expected_release_menu().unwrap();
        assert_eq!(table, expected_menu);
    }

    #[test]
    fn the_wall_2_indices() {
        let album = TestAlbum::TheWallDisc2;
        let toc: cdtoc::Toc = album.expected_toc();
        let tracks: Vec<Track> = album.expected_tracks_minimal();
        let leadout = album.expected_leadout();
        let mut disc = Disc::new(toc, tracks, leadout).unwrap();
        let musicbrainz = album.expected_musicbrainz();
        disc.set_musicbrainz(musicbrainz);

        let releasemenu = release_menu(&disc).unwrap();

        // reverse() -> identical dates end up in reverse index order
        let orignal_indices = [1, 3, 0, 7, 6, 5, 2, 4];
        let indices: Vec<_> = (1..=8)
            .map(|selection| releasemenu.index_for(selection))
            .collect();
        assert_eq!(indices, orignal_indices);
    }
}
