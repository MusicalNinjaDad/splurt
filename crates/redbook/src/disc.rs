use std::{
    fs::File,
    io::{self, Write},
    path::{Path, PathBuf},
};

use bytes::Bytes;
use cdtoc::Toc;
use metaflac::block::VorbisComment;
use musicbrainz_rs::{
    Fetch, MusicBrainzClient,
    api_bindium::{ApiClient, ureq},
};

use crate::{
    Frame, Msf, Track,
    musicbrainz::{ArtistCreditsExt, Discid, Release, VorbisTagExt},
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
    coverart: Option<Bytes>,
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
            .and_then(|release| release.artist_credit.names().nth(0))
    }

    /// Returns an *owned* Option<Track> with metadata valid for 'self
    ///
    /// - Holding on to the returned track will block any mutation to Self, in order
    ///   to maintain validity of the metadata.
    /// - Modifying the returned track will NOT modify the copy stored in Self
    pub fn track(&self, track_number: usize) -> Option<Track<'_>> {
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
        Tracks { disc: self, i: 0 }
    }

    /// Use the release at the given index, or reset selection to None.
    /// Providing an invalid index will make no change.
    ///
    /// Returns a reference to the release set, to allow for validation.
    pub fn set_release(&mut self, index: Option<usize>) -> &mut Self {
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
        self.disc_index = match self.release()?.media.as_ref()?.len() {
            ..=1 => Some(0),
            _ => None,
        };
        self.disc_index
    }

    /// Provides an iterator over the releases to allow for programatic selection.
    ///
    /// Takes a closure which accepts a slice of releases and returns the index of
    /// the release to select.
    pub fn select_release<F>(&mut self, selector: F) -> &mut Self
    where
        F: FnOnce(&[Release]) -> Option<usize>,
    {
        if let Some(Some(releases)) = self.musicbrainz.as_ref().map(|disc_id| &disc_id.releases) {
            self.set_release(selector(releases));
        }
        self
    }

    /// Attempt to update the data from musicbrainz
    pub fn update_musicbrainz(&mut self) -> io::Result<()> {
        let discid = self.toc.musicbrainz_id().to_string();

        let native_tls = ureq::tls::TlsConfig::builder();
        let native_tls = native_tls
            .provider(ureq::tls::TlsProvider::NativeTls)
            .build();
        dbg!(&native_tls);

        let agent = ureq::Agent::config_builder();
        let agent = agent
            .tls_config(native_tls)
            .user_agent("splurt_musicbrainz_rs/0.1.0")
            .build()
            .new_agent();
        dbg!(&agent);

        let api_client = ApiClient::builder();
        let api_client = api_client.agent(agent).build();
        dbg!(&api_client);

        let mb_client = MusicBrainzClient::builder();
        let mb_client = mb_client.api_client(api_client).build();
        dbg!(&mb_client);

        let mut mb_stuff = Discid::fetch();
        mb_stuff.id(&discid).with_artists().with_recordings();

        let api_call = mb_stuff.as_api_request(&mb_client).unwrap();
        dbg!(api_call.uri());
        dbg!(api_call.headers());
        dbg!(api_call.body());

        self.musicbrainz = Some(
            mb_stuff
                .execute_with_client(&mb_client)
                .map_err(io::Error::other)?,
        );
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
            .get(url)
            .header("User-Agent", "splurt/0.1.0")
            .send()
            .map_err(io::Error::other)?;
        if response.status().is_success() {
            let image = response.bytes().map_err(io::Error::other)?;
            self.coverart = Some(image);
        } else {
            dbg!(response);
        }
        Ok(())
    }

    /// Get the cached cover art
    pub fn cover_art(&self) -> Option<&[u8]> {
        self.coverart.as_ref().map(|b| b.as_ref())
    }

    /// Save the cover art as "front.jpeg"
    ///
    /// Returns None if no cover art is available, else the absolute location of the saved file.
    pub fn save_cover_art<P: AsRef<Path>>(&self, directory: P) -> Option<io::Result<PathBuf>> {
        let data = self.cover_art()?;
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
        let mut vorbis = VorbisComment::new();
        let track = self.track(track_number)?;

        vorbis.set_track(track.track_number() as u32);

        if let Some(id) = track.windows_identifier {
            vorbis.set("WINDOWS_IDENTIFIER", vec![id.to_string()])
        }

        if let Some(release) = self.release() {
            vorbis.set_album(vec![release.title.clone()]);
            vorbis.set("MUSICBRAINZ_ALBUMID", vec![release.id.clone()]);

            vorbis.set_album_artist(release.artist_credit.names().collect());
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
                vorbis.set_artist(track_artists.names().collect());

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

#[cfg(test)]
mod tests {
    use super::*;

    mod definitely_maybe {
        use super::*;
        use crate::{Msf, TocEntry, hex::*};

        fn create_toc() -> Toc {
            let toc_dump = hex_to_bytes(include_str!(
                "../tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/TOC.hex"
            ))
            .unwrap();
            let toc_string = parse_toc(toc_dump);
            Toc::from_cdtoc(toc_string).unwrap()
        }

        fn create_tracks() -> Vec<Track<'static>> {
            vec![
                Track {
                    toc_entry: TocEntry {
                        track: 1,
                        start: Frame::from(Msf::new(0x00, 0x02, 0x21)),
                    },
                    duration_frames: Frame::new(24242),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 2,
                        start: Frame::from(Msf::new(0x05, 0x19, 0x32)),
                    },
                    duration_frames: Frame::new(23138),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 3,
                        start: Frame::from(Msf::new(0x0A, 0x22, 0x0D)),
                    },
                    duration_frames: Frame::new(20762),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 4,
                        start: Frame::from(Msf::new(0x0F, 0x0B, 0x00)),
                    },
                    duration_frames: Frame::new(20168),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 5,
                        start: Frame::from(Msf::new(0x13, 0x27, 0x44)),
                    },
                    duration_frames: Frame::new(28272),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 6,
                        start: Frame::from(Msf::new(0x19, 0x38, 0x41)),
                    },
                    duration_frames: Frame::new(21280),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 7,
                        start: Frame::from(Msf::new(0x1E, 0x28, 0x2D)),
                    },
                    duration_frames: Frame::new(19338),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 8,
                        start: Frame::from(Msf::new(0x22, 0x3A, 0x21)),
                    },
                    duration_frames: Frame::new(21700),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 9,
                        start: Frame::from(Msf::new(0x27, 0x2F, 0x3A)),
                    },
                    duration_frames: Frame::new(11425),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 10,
                        start: Frame::from(Msf::new(0x2A, 0x14, 0x08)),
                    },
                    duration_frames: Frame::new(29455),
                    ..Default::default()
                },
                Track {
                    toc_entry: TocEntry {
                        track: 11,
                        start: Frame::from(Msf::new(0x30, 0x34, 0x3F)),
                    },
                    duration_frames: Frame::new(14440),
                    ..Default::default()
                },
            ]
        }

        fn create_disc() -> Disc {
            Disc {
                toc: create_toc(),
                tracks: create_tracks(),
                leadout: Frame::from(Msf::new(0x34, 0x05, 0x1C)),
                musicbrainz: None,
                release_index: None,
                disc_index: None,
                coverart: None,
            }
        }

        #[test]
        fn new() {
            let toc = create_toc();
            let tracks = create_tracks();
            let leadout = Frame::from(Msf::new(0x34, 0x05, 0x1C));

            let disc = Disc::new(toc, tracks, leadout).unwrap();

            let expected = create_disc();
            assert_eq!(disc, expected);
        }

        #[test]
        fn set_release() {
            let mut disc = create_disc();
            let json = include_str!(
                "../tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/musicbrainz_disc.json"
            );
            disc.musicbrainz = Some(serde_json::from_str(json).unwrap());

            disc.set_release(Some(2));
            assert_eq!(disc.release_index, Some(2));
        }

        #[test]
        fn set_release_no_musicbrainz() {
            let mut disc = create_disc();

            disc.set_release(Some(0));
            assert!(disc.release_index.is_none());
        }

        #[test]
        fn set_release_invalid() {
            let mut disc = create_disc();
            let json = include_str!(
                "../tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/musicbrainz_disc.json"
            );
            disc.musicbrainz = Some(serde_json::from_str(json).unwrap());

            disc.set_release(Some(999));
            assert!(disc.release_index.is_none());
        }

        #[test]
        fn track() {
            let mut disc = create_disc();
            let json = include_str!(
                "../tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/musicbrainz_disc.json"
            );
            disc.musicbrainz = Some(serde_json::from_str(json).unwrap());

            disc.set_release(Some(2));
            let columbia = disc.track(5).unwrap();
            assert_eq!(columbia.title(), Some("Columbia".to_string()));
        }

        #[test]
        fn tracks_len() {
            let mut disc = create_disc();
            let json = include_str!(
                "../tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/musicbrainz_disc.json"
            );
            disc.musicbrainz = Some(serde_json::from_str(json).unwrap());

            disc.set_release(Some(2));
            assert_eq!(disc.tracks().len(), 11);
        }

        #[test]
        fn tracks_data() {
            let mut disc = create_disc();
            let json = include_str!(
                "../tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/musicbrainz_disc.json"
            );
            disc.musicbrainz = Some(serde_json::from_str(json).unwrap());

            disc.set_release(Some(2));
            let columbia = disc.tracks().nth(4).unwrap();
            assert_eq!(columbia.title(), Some("Columbia".to_string()));
        }
    }
}
