//! Disc metadata and MusicBrainz integration
//!
//! # Tracing
//!
//! This module emits the following spans:
//! - `Disc::new` (INFO): Disc creation with `track_count` and `album_name` fields
//! - `Disc::track` (DEBUG): Track lookup with `track_number` field
//! - `Disc::tracks` (DEBUG): Track iteration
//! - `Disc::set_release` (DEBUG): Release selection with `index` field
//! - `Disc::tag_for` (DEBUG): Tag generation with `track_number` and `title` fields
//! - `Disc::update_musicbrainz` (INFO): MusicBrainz update with `discid` field
//! - `Disc::update_cover_art` (INFO): Cover art retrieval
//!
//! Events:
//! - `musicbrainz_retrieved` (INFO): On successful MusicBrainz lookup with `releases` count
//! - `musicbrainz_failed` (ERROR): On MusicBrainz lookup failure with `error` field
//! - `coverart_retrieved` (INFO): On successful cover art retrieval with `size_bytes` field
//! - `coverart_failed` (WARN): On cover art retrieval failure with `url`, `status`, and `reason` fields

use std::{
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

use tracing::info;

use cdtoc::Toc;
use metaflac::block::{Picture, PictureType, VorbisComment};
use musicbrainz_rs::{
    Fetch, MusicBrainzClient,
    api_bindium::{ApiClient, ureq},
};

use crate::{
    Frame, Msf, Track,
    musicbrainz::{ArtistCreditsExt, Discid, Release, VorbisTagExt},
    tagging::PictureExt,
};

#[derive(Debug, Clone, PartialEq)]
/// Logically: a physical CD.
///
/// This is the main starting point for all data and actions you take on the CD itself.
/// It is usually stored in some kind of drive struct which implements
/// [`AudioCdExt`][crate::AudioCdExt] and therefore knows how to get data from the CD.
pub struct Disc {
    toc: Toc,
    tracks: Vec<Track<'static>>,
    leadout: Frame,
    musicbrainz: Option<Discid>,
    /// Selected release index from musicbrainz.releases. Use [`select_release()`]
    /// or [set_release()] to set and [`release()`] to get.
    ///
    /// - None if no selection made yet
    /// - Some(0) if no data present
    /// - Some(0) if first release selected
    /// - Some(n) if specific release selected
    release_index: Option<usize>,
    /// The 0-indexed disc number. Needed for multi-disc releases.
    /// Automatically set to Some(0) for single-disc releases.
    ///
    /// - None if no release is selected
    /// - Some(n) if release is selected
    disc_index: Option<usize>,
    /// Cached coverart: if available
    coverart: Option<Picture>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscError {
    IncorrectLeadout,
    TocMismatch,
}

impl std::fmt::Display for DiscError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscError::IncorrectLeadout => write!(f, "incorrect leadout"),
            DiscError::TocMismatch => write!(f, "TOC mismatch"),
        }
    }
}

impl std::error::Error for DiscError {}

impl From<DiscError> for std::io::Error {
    fn from(error: DiscError) -> Self {
        std::io::Error::new(std::io::ErrorKind::InvalidData, error)
    }
}

impl Disc {
    pub fn new<T: IntoIterator<Item = Track<'static>>>(
        toc: Toc,
        tracks: T,
        leadout: Frame,
    ) -> Result<Self, DiscError> {
        let tracks: Vec<_> = tracks.into_iter().collect();
        let _span = tracing::info_span!("Disc::new", track_count = tracks.len());
        let _enter = _span.enter();

        if toc.leadout() != leadout.as_usize() as u32 {
            return Err(DiscError::IncorrectLeadout);
        }

        for track in tracks.iter() {
            let track_number = track.toc_entry.track as usize;
            let toc_track = toc
                .audio_track(track_number)
                .ok_or(DiscError::TocMismatch)?;

            let (min, sec, frame) = toc_track.msf();
            if Msf::new(min as i8, sec as i8, frame as i8) != Msf::from(track.toc_entry.start) {
                return Err(DiscError::TocMismatch);
            }

            let (d, h, min, sec, frame) = toc_track.duration().dhmsf();
            let min = (((d * 24) + h as u64) * 60) + min as u64;
            if Msf::new(min as i8, sec as i8, frame as i8) != Msf::from(track.duration_frames) {
                return Err(DiscError::TocMismatch);
            }
        }

        Ok(Self {
            toc,
            tracks,
            leadout,
            musicbrainz: None,
            release_index: None,
            disc_index: None,
            coverart: None,
        })
    }

    /// Get the selected release
    pub fn release(&self) -> Option<&Release> {
        self.musicbrainz
            .as_ref()?
            .releases
            .as_ref()?
            .get(self.release_index?)
    }

    /// Get the full MusicBrainz data
    pub fn musicbrainz(&self) -> Option<&Discid> {
        self.musicbrainz.as_ref()
    }

    /// Get the title of the CD
    pub fn title(&self) -> Option<String> {
        self.release().map(|release| release.title.clone())
    }

    pub fn main_artist(&self) -> Option<String> {
        self.release()
            .and_then(|release| release.artist_credit.main_artist())
    }

    /// Returns an *owned* Option<Track> with metadata valid for 'self
    ///
    /// - Holding on to the returned track will block any mutation to Self, in order
    ///   to maintain validity of the metadata.
    /// - Modifying the returned track will NOT modify the copy stored in Self
    pub fn track(&self, track_number: usize) -> Option<Track<'_>> {
        let _span = tracing::debug_span!("Disc::track", track_number = track_number);
        let _enter = _span.enter();
        let mut track = self.tracks.get(track_number - 1).cloned()?;
        track.meta = self
            .release()
            .and_then(|r| r.media.as_ref())
            .and_then(|all_media| all_media.get(self.disc_index?))
            .and_then(|media| media.tracks.as_ref())
            .and_then(|tracks| {
                tracks
                    .iter()
                    .find(|trk| trk.number.parse() == Ok(track_number))
            });
        Some(track)
    }

    pub fn tracks(&self) -> Tracks<'_> {
        let _span = tracing::debug_span!("Disc::tracks", track_count = self.tracks.len());
        let _enter = _span.enter();
        Tracks { disc: self, i: 0 }
    }

    /// Use the release at the given index, or reset selection to None.
    /// Providing an invalid index will make no change.
    ///
    /// Returns a reference to the release set, to allow for validation.
    pub fn set_release(&mut self, index: Option<usize>) -> &mut Self {
        let _span = tracing::debug_span!("Disc::set_release", index = ?index);
        let _enter = _span.enter();
        self.release_index = match index {
            Some(index)
                if self
                    .musicbrainz
                    .as_ref()
                    .and_then(|disc_id| disc_id.releases.as_ref())
                    .and_then(|release| release.get(index))
                    .is_some() =>
            {
                Some(index)
            }
            _ => None,
        };
        let _ = self.reset_disc_index();
        self
    }

    /// Reset the disc index to None (multi-disc / unknown release) / Some(0) (single-disc)
    pub fn reset_disc_index(&mut self) -> Option<usize> {
        let release = self.release()?;
        let media = release.media.as_ref()?;

        self.disc_index = match media.len() {
            ..=1 => Some(0),
            _ => self.find_disc_index_from_media(),
        };
        self.disc_index
    }

    /// Find which media entry in the release matches this disc's TOC
    ///
    /// For multi-disc releases, each media entry represents one disc.
    /// We match by comparing track offsets from the Discid with calculated offsets from media.
    ///
    /// Returns:
    /// - Some(index) if exactly one media entry matches based on track offsets
    /// - None if no matches or multiple matches (ambiguous)
    fn find_disc_index_from_media(&self) -> Option<usize> {
        let release = self.release()?;
        let offsets = self.toc.audio_sectors();
        let matches: Vec<_> = release
            .media
            .as_ref()
            .map(|all_media| {
                all_media.iter().filter(|media| {
                    media
                        .discs
                        .as_ref()
                        .and_then(|discs| discs.iter().find(|disc| disc.offsets == offsets))
                        .is_some()
                })
            })?
            .collect();

        match matches.len() {
            1 => release.media.as_ref().and_then(|all_media| {
                all_media.iter().position(|media| {
                    Some(&media.id) == matches.first().map(|matched_media| &matched_media.id)
                })
            }),
            _ => None,
        }
    }

    /// Set the MusicBrainz data directly
    pub fn set_musicbrainz(&mut self, discid: Discid) -> &mut Self {
        self.musicbrainz = Some(discid);
        self.release_index = match self
            .musicbrainz
            .as_ref()
            .and_then(|mb| mb.releases.as_ref())
        {
            None => Some(0),
            Some(releases) if releases.is_empty() => Some(0),
            Some(releases) if releases.len() == 1 => Some(1),
            _ => None,
        };
        self
    }

    /// Attempt to update the data from musicbrainz
    pub fn update_musicbrainz(&mut self) -> io::Result<()> {
        let discid = self.toc.musicbrainz_id().to_string();
        let _span = tracing::info_span!("update_musicbrainz", discid = %discid);
        let _enter = _span.enter();

        let native_tls = ureq::tls::TlsConfig::builder();
        let native_tls = native_tls
            .provider(ureq::tls::TlsProvider::NativeTls)
            .build();

        let agent = ureq::Agent::config_builder();
        let agent = agent
            .tls_config(native_tls)
            .user_agent("splurt_musicbrainz_rs/0.1.0")
            .build()
            .new_agent();

        let api_client = ApiClient::builder();
        let api_client = api_client.agent(agent).build();

        let mb_client = MusicBrainzClient::builder();
        let mb_client = mb_client.api_client(api_client).build();

        let mut mb_stuff = Discid::fetch();
        mb_stuff.id(&discid).with_artists().with_recordings();

        let _api_call = mb_stuff.as_api_request(&mb_client).unwrap();

        let discid = mb_stuff
            .execute_with_client(&mb_client)
            .map_err(io::Error::other)?;

        self.set_musicbrainz(discid);

        if let Some(ref mb) = self.musicbrainz {
            let release_count = mb.releases.as_ref().map(|r| r.len()).unwrap_or(0);
            info!(releases = release_count, "musicbrainz_retrieved");
        }
        Ok(())
    }

    /// Attempt to get the front cover art from the CoverArtArchive.
    pub fn update_cover_art(&mut self) -> io::Result<()> {
        let release_mbid = self
            .release()
            .ok_or_else(|| io::Error::other("No releases found"))?
            .id
            .clone();

        let client = reqwest::blocking::Client::new();
        let url = format!("https://coverartarchive.org/release/{release_mbid}/front");
        let response = client
            .get(&url)
            .header("User-Agent", "splurt/0.1.0")
            .send()
            .map_err(io::Error::other)?;

        let _span = tracing::info_span!("update_cover_art", url = %url);
        let _enter = _span.enter();

        if response.status().is_success() {
            let image = response.bytes().map_err(io::Error::other)?;
            let cover = Picture::from_jpeg(PictureType::CoverFront, "Front Cover", image.clone());
            self.coverart = Some(cover);
            info!(size_bytes = image.len(), "coverart_retrieved");
        } else {
            let status = response.status();
            let reason = response.text().ok();
            tracing::warn!(url = %url, status = %status, reason = ?reason, "coverart_failed");
        }
        Ok(())
    }

    /// Get the cached cover art
    pub fn cover_art(&self) -> Option<&Picture> {
        self.coverart.as_ref()
    }

    /// Get the 0-indexed disc number for multi-disc releases
    pub fn disc_index(&self) -> Option<usize> {
        self.disc_index
    }

    /// Save the cover art as "front.jpeg"
    ///
    /// Returns None if no cover art is available, else the absolute location of the saved file.
    #[must_use = "may be `Some(Err(_))`"]
    pub fn save_cover_art<P: AsRef<Path>>(&self, directory: P) -> Option<io::Result<PathBuf>> {
        let data = &self.cover_art()?.data;
        let written_to_path = try {
            let path = directory
                .as_ref()
                .to_owned()
                .join("front.jpeg")
                .absolute()?;
            let mut cover = File::create_new(&path)?;
            cover.write_all(data)?;
            path
        };
        Some(written_to_path)
    }

    /// Will only be None if given an invalid track number.
    /// Otherwise at least "TRACKNUMBER" will be set.
    pub fn tag_for(&self, track_number: usize) -> Option<VorbisComment> {
        let _span = tracing::debug_span!("Disc::tag_for", track_number = track_number);
        let _enter = _span.enter();
        let mut vorbis = VorbisComment::new();
        let track = self.track(track_number)?;

        vorbis.set_track(track.track_number() as u32);

        if let Some(id) = track.windows_identifier {
            vorbis.set("WINDOWS_IDENTIFIER", vec![id.to_string()])
        }

        if let Some(release) = self.release() {
            vorbis.set_album(vec![release.title.clone()]);
            vorbis.set("MUSICBRAINZ_ALBUMID", vec![release.id.clone()]);

            vorbis.set_album_artist(release.artist_credit.artist_names().collect());
            vorbis.set(
                "MUSICBRAINZ_ALBUMARTISTID",
                release.artist_credit.artist_ids().collect(),
            );

            let release_date = release
                .date
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            let release_year = release
                .date
                .as_ref()
                .and_then(|date| date.year())
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            vorbis.set("RELEASEDATE", vec![release_date]);
            vorbis.set("RELEASEYEAR", vec![release_year]);

            vorbis.set(
                "RELEASECOUNTRY",
                vec![release.country.clone().unwrap_or_default()],
            );
            release.status.as_ref().unwrap().extend_vorbis(&mut vorbis);

            vorbis.set("BARCODE", vec![release.barcode.clone().unwrap_or_default()]);

            if let Some(media_list) = release.media.as_ref() {
                let total_discs = media_list.len();
                vorbis.set("TOTALDISCS", vec![total_discs.to_string()]);
                vorbis.set("DISCTOTAL", vec![total_discs.to_string()]);
                if let Some(track_count) = media_list.first().map(|media| media.track_count) {
                    vorbis.set("TOTALTRACKS", vec![track_count.to_string()]);
                    vorbis.set("TRACKTOTAL", vec![track_count.to_string()]);
                };
            }
            if let Some(disc_number) = self.disc_index {
                vorbis.set("DISCNUMBER", vec![(disc_number + 1).to_string()]);
            }
            vorbis.set(
                "MEDIA",
                vec![
                    release
                        .media
                        .as_ref()
                        .and_then(|all_media| all_media.first())
                        .and_then(|media| media.format.clone())
                        .unwrap_or_default(),
                ],
            );

            release
                .text_representation
                .as_ref()
                .and_then(|text_rep| text_rep.script.as_ref())
                .unwrap()
                .extend_vorbis(&mut vorbis);

            if let Some(meta) = track.meta() {
                vorbis.set_title(vec![track.title().clone().unwrap_or_default()]);

                vorbis.set("MUSICBRAINZ_TRACKID", vec![meta.id.clone()]);

                let track_artists = meta
                    .artist_credit
                    .as_ref()
                    .or(release.artist_credit.as_ref());
                vorbis.set_artist(track_artists.artist_names().collect());

                let original_date = meta
                    .recording
                    .as_ref()
                    .and_then(|recording| recording.first_release_date.clone())
                    .unwrap_or_default();
                let original_year = original_date.year().unwrap_or_default().to_string();
                vorbis.set("ORIGINALDATE", vec![original_date]);
                vorbis.set("ORIGINALYEAR", vec![original_year]);
            }
        }

        Some(vorbis)
    }
}

#[derive(Debug)]
pub struct Tracks<'meta> {
    disc: &'meta Disc,
    i: usize,
}

impl<'meta> Iterator for Tracks<'meta> {
    type Item = Track<'meta>;

    fn next(&mut self) -> Option<Self::Item> {
        // disc.track() uses 1-indexing so we can be very lazy
        self.i += 1;
        self.disc.track(self.i)
    }
}

impl<'m> ExactSizeIterator for Tracks<'m> {
    fn len(&self) -> usize {
        self.disc.tracks.len()
    }

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(all(test, feature = "test_fixtures"))]
mod tests {
    use super::*;
    use crate::test_fixtures::albums::TestAlbum::{self, *};
    use rstest::rstest;

    // TODO - validate with minimal track info

    #[rstest]
    #[case(DefinitelyMaybe)]
    #[case(TheWallDisc1)]
    #[case(TheWallDisc2)]
    fn new(#[case] album: TestAlbum) {
        let toc = album.expected_toc();
        let tracks = album.expected_tracks_minimal();
        let leadout = album.expected_leadout();

        let disc = Disc::new(toc, tracks, leadout).unwrap();
        assert_eq!(disc.toc, album.expected_toc());
        assert_eq!(
            disc.tracks().collect::<Vec<_>>(),
            album.expected_tracks_minimal()
        );
        assert_eq!(disc.leadout, album.expected_leadout());
    }

    #[rstest]
    #[case(DefinitelyMaybe)]
    #[case(TheWallDisc1)]
    #[case(TheWallDisc2)]
    fn identify_disc_index(#[case] album: TestAlbum) {
        let toc = album.expected_toc();
        let tracks = album.expected_tracks_minimal();
        let leadout = album.expected_leadout();
        let musicbrainz = album.expected_musicbrainz();

        let mut disc = Disc::new(toc, tracks, leadout).unwrap();
        disc.set_musicbrainz(musicbrainz);
        disc.set_release(Some(album.release()));
        assert_eq!(disc.disc_index(), album.expected_disc_index());
    }

    #[test]
    fn set_release_invalid() {
        let album = DefinitelyMaybe;
        let toc = album.expected_toc();
        let tracks = album.expected_tracks_minimal();
        let leadout = album.expected_leadout();
        let musicbrainz = album.expected_musicbrainz();

        let mut disc = Disc::new(toc, tracks, leadout).unwrap();
        disc.set_musicbrainz(musicbrainz);

        disc.set_release(Some(999));
        assert!(disc.release_index.is_none());
    }

    #[test]
    fn set_release_no_musicbrainz() {
        let album = DefinitelyMaybe;
        let toc = album.expected_toc();
        let tracks = album.expected_tracks_minimal();
        let leadout = album.expected_leadout();

        let mut disc = Disc::new(toc, tracks, leadout).unwrap();

        disc.set_release(Some(0));
        assert!(disc.release_index.is_none());
    }
}
