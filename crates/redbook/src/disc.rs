use std::io;

use bytes::Bytes;
use cdtoc::Toc;

use crate::{
    Frame, Msf, Track,
    musicbrainz::{DiscId, Release},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// Logically: a physical CD.
///
/// This is the main starting point for all data and actions you take on the CD itself.
/// It is usually stored in some kind of drive struct which implements
/// [`AudioCdExt`][crate::AudioCdExt] and therefore knows how to get data from the CD.
pub struct Disc {
    toc: Toc,
    tracks: Vec<Track<'static>>,
    leadout: Frame,
    musicbrainz: Option<DiscId>,
    /// Selected release index from musicbrainz.releases. Use [`select_release()`]
    /// or [set_release()] to set and [`release()`] to get.
    ///
    /// - None if no selection made yet
    /// - Some(0) if no data present
    /// - Some(0) if first release selected
    /// - Some(n) if specific release selected
    release_index: Option<usize>,
    /// The 0-indexed disc number. Needed for multi-disc releases.
    /// Will be None or Some(0) for single-disc releases.
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
        self.musicbrainz.as_ref()?.releases.get(self.release_index?)
    }

    /// Get the MusicBrainz data
    pub fn musicbrainz(&self) -> Option<&DiscId> {
        self.musicbrainz.as_ref()
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
                    .and_then(|disc_id| disc_id.releases.get(index))
                    .is_some() =>
            {
                Some(index)
            }
            _ => None,
        };
        self
    }

    /// Provides an iterator over the releases to allow for programatic selection.
    ///
    /// Takes a closure which accepts a slice of releases and returns the index of
    /// the release to select.
    pub fn select_release<F>(&mut self, selector: F) -> &mut Self
    where
        F: FnOnce(&[Release]) -> Option<usize>,
    {
        if let Some(releases) = self.musicbrainz.as_ref().map(|disc_id| &disc_id.releases) {
            self.set_release(selector(releases));
        }
        self
    }

    /// Attempt to update the data from musicbrainz
    pub fn update_musicbrainz(&mut self) -> io::Result<()> {
        let discid = self.toc.musicbrainz_id().to_string();
        let url = format!("https://musicbrainz.org/ws/2/discid/{discid}?inc=recordings&fmt=json");

        let client = reqwest::blocking::Client::new();
        self.musicbrainz = Some(
            client
                .get(url)
                .header("User-Agent", "splurt/0.1.0")
                .send()
                .map_err(io::Error::other)?
                .json::<DiscId>()
                .map_err(io::Error::other)?,
        );
        self.release_index = match self.musicbrainz {
            None => Some(0),
            Some(ref mb_data) if mb_data.releases.is_empty() => Some(0),
            Some(ref mb_data) if mb_data.releases.len() == 1 => Some(1),
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
    }
}
