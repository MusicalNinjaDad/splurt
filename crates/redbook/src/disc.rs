use bytes::Bytes;
use cdtoc::Toc;

use crate::{Frame, Msf, Track, musicbrainz::DiscId};

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
        
        (toc.leadout() == leadout.as_usize() as u32).ok_or(DiscError::IncorrectLeadout)?;
        
        for track in tracks.iter() {
            let track_number = track.toc_entry.track as usize;
            let toc_track = toc
                .audio_track(track_number)
                .ok_or(DiscError::TocMismatch)?;
            let (min, sec, frame) = toc_track.msf();
            (Msf::new(min as i8, sec as i8, frame as i8) == Msf::from(track.toc_entry.start))
                .ok_or(DiscError::TocMismatch)?;
            let (d, h, min, sec, frame) = toc_track.duration().dhmsf();
            let min = (((d * 24) + h as u64) * 60) + min as u64;
            (Msf::new(min as i8, sec as i8, frame as i8) == Msf::from(track.duration_frames))
                .ok_or(DiscError::TocMismatch)?;
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
}
