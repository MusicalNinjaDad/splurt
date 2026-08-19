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
    tracks: Vec<Track>,
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

impl Disc {
    pub fn new<T: IntoIterator<Item = Track>>(
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
    pub fn update_musicbrainz(&mut self) -> io::Result<&mut Self> {
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
        Ok(self)
    }

    /// Attempt to get the front cover art from the CoverArtArchive.
    pub fn update_cover_art(&mut self) -> io::Result<&mut Self> {
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
        Ok(self)
    }
}
