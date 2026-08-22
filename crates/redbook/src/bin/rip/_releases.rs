use redbook::{Disc, musicbrainz::Release};
use std::collections::BTreeMap;

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
        let title = release.title.as_str();
        groups.entry(title).or_default().push(release);
    }

    let mut output = String::new();
    output.push_str("Multiple releases found. Please select one:\n\n");

    let mut index = 1;
    for (group_title, group_releases) in &groups {
        // Group header
        output.push_str(group_title);
        output.push_str("\n");

        // Underline
        let underline = "=".repeat(group_title.len());
        output.push_str(&underline);
        output.push_str("\n");

        // Table header
        output.push_str("    Date        Country     Barcode\n");

        // Sort releases within group by date descending (newest first) for consistent ordering
        let mut sorted_releases = group_releases.to_vec();
        sorted_releases.sort_by(|a, b| {
            let a_date = a.date.as_ref().map(|d| d.to_string().trim().to_string()).unwrap_or_default();
            let b_date = b.date.as_ref().map(|d| d.to_string().trim().to_string()).unwrap_or_default();
            b_date.cmp(&a_date)
        });

        for release in sorted_releases {
            // Index: 4 chars (e.g., "1.  ", "10. ")
            output.push_str(&format!("{}.  ", index));
            index += 1;

            // Date: right-aligned in 10-char width at positions 4-13
            let date = release
                .date
                .as_ref()
                .map(|d| d.to_string().trim().to_string())
                .unwrap_or_default();
            output.push_str(&format!("{:<10}", date));

            // 2 spaces at positions 14-15
            output.push_str("  ");

            // Country: left-aligned in 10-char field at positions 16-25
            let country = release.country.as_deref().unwrap_or("");
            output.push_str(&format!("{:<10}", country));

            // Barcode: starts at position 26, no padding
            let barcode = release.barcode.as_deref().unwrap_or("");
            output.push_str(barcode);

            // Disambiguation if present: 4 spaces before parentheses
            if let Some(disambig) = &release.disambiguation {
                if !disambig.is_empty() {
                    output.push_str("    ");
                    output.push_str(&format!("({})", disambig));
                }
            }

            output.push_str("\n");
        }

        output.push_str("\n");
    }

    Some(output)
}

#[cfg(all(test, feature = "test_fixtures"))]
mod tests {
    use redbook::{Track, test_fixtures::albums::TestAlbum};

    use super::*;

    #[test]
    fn the_wall_2() {
        let album = TestAlbum::TheWallDisc2;
        let toc: cdtoc::Toc = album.expected_toc();
        let tracks: Vec<Track> = album.expected_tracks();
        let leadout = album.expected_leadout();
        let mut disc = Disc::new(toc, tracks, leadout).unwrap();
        let musicbrainz: redbook::musicbrainz::Discid = album.expected_musicbrainz();
        disc.set_musicbrainz(musicbrainz);
        let release_selection = release_menu(disc);
        let expected_menu = album.expected_release_menu();
        assert_eq!(release_selection, expected_menu);
    }
}
