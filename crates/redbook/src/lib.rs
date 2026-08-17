#![feature(exact_div)]
#![feature(try_blocks)]
// Unsafe restricted to dedicated wrapper modules
#![deny(unsafe_code)]
#![forbid(unsafe_op_in_unsafe_fn)]
#![forbid(unsafe_attr_outside_unsafe)]

//! CDDA CD digital audio as per RedBook (IEC 60908:1999)
//!
//! Frame IDs are always *absolute* and *include* the lead-in (150 frames)
//! Timestamps are always *relative* to the start of the audio and *exclude* the lead-in (2s)

pub mod musicbrainz;
pub mod win;

pub use win::AudioCd;

use std::{convert::TryFrom, io};

use cdtoc::{Toc, TocError};
use musicbrainz::DiscId;

use crate::musicbrainz::Release;

/// One cdda audio frame in bytes
const FRAME_SIZE: usize = 2352;

// If chunks are too large DeviceIoControl(.., IOCTL_CDROM_RAW_READ,..) fails.
// Calc frames first, then reverse calc bytes as we need an exact number of frames.
// TODO: research max chunk size. Guessing 64k for now based on something I saw in cd_da_reader but with no references given
const MAX_CHUNK_FRAMES: usize = 64 * 1024 / FRAME_SIZE;
const MAX_CHUNK_BYTES: usize = MAX_CHUNK_FRAMES * FRAME_SIZE;

/// Functions common to redbook audio CDs.
pub trait AudioCdExt {
    /// Obtain Track details
    fn track(&self, track_number: usize) -> Option<&Track>;

    /// Get all tracks from the CD
    fn tracks(&self) -> impl Iterator<Item = &Track>;

    /// Frame address for leadout
    fn leadout(&self) -> u32;

    /// Reads `frames_to_read` worth of data starting at `track` + `frame_offset` into `buf`
    ///
    /// Returns the number of bytes read
    fn read_chunk(
        &self,
        track: &Track,
        frame_offset: usize,
        frames_to_read: u32,
        buf: &mut [u8],
    ) -> io::Result<u32>;

    /// Return a [Toc] for the Cd
    fn toc(&self) -> Result<Toc, TocError>;

    /// Return a reference to the cached Disc data
    fn disc(&self) -> &Disc;

    /// Return a mutable reference to the cached Disc data
    fn disc_mut(&mut self) -> &mut Disc;

    /// Read a full track, returning the raw data as a `Vec` of bytes.
    fn read_track(&self, track_number: usize) -> io::Result<Vec<u8>> {
        let track = self.track(track_number).unwrap();
        dbg!(track);
        let track_size = usize::try_from(track.duration_frames)
            .unwrap()
            .strict_mul(FRAME_SIZE);
        debug_assert!(track_size > 0);

        // Vec needs to be initialised to split into chunks. Performance cost insignificant vs IO.
        let mut data = vec![0_u8; track_size];
        dbg!(data.len());

        // TODO: Handle very short tracks < MAX_CHUNK_FRAMES
        let (bufs, last_buf) = data.as_chunks_mut::<MAX_CHUNK_BYTES>();
        let mut bytes_read_so_far = 0_i64;

        for (i, buf) in bufs.iter_mut().enumerate() {
            let frames_to_read: u32 = MAX_CHUNK_FRAMES.try_into().unwrap();

            debug_assert_eq!(
                bytes_read_so_far,
                (i as i64).strict_mul(MAX_CHUNK_BYTES as i64),
                "now reading chunk {i} but have only read {bytes_read_so_far} bytes so far"
            );

            let frame_offset = i * MAX_CHUNK_FRAMES;
            debug_assert_eq!(
                i64::try_from(frame_offset)
                    .unwrap()
                    .strict_mul(FRAME_SIZE.try_into().unwrap()),
                bytes_read_so_far,
                "about to read chunk {i}. We have read {frame_offset} frames, but only {bytes_read_so_far} bytes so far"
            );

            let bytes_read = self.read_chunk(track, frame_offset, frames_to_read, buf)?;
            bytes_read_so_far += i64::from(bytes_read);
        }

        let frame_offset = bufs.len().strict_mul(MAX_CHUNK_FRAMES);
        debug_assert_eq!(
            i64::try_from(frame_offset)
                .unwrap()
                .strict_mul(FRAME_SIZE.try_into().unwrap()),
            bytes_read_so_far,
            "about to read last chunk. We have read {frame_offset} frames, but only {bytes_read_so_far} bytes so far"
        );
        let frames_to_read = track
            .duration_frames
            .strict_rem(MAX_CHUNK_FRAMES.try_into().unwrap());

        let bytes_read = self.read_chunk(track, frame_offset, frames_to_read, last_buf)?;
        bytes_read_so_far += i64::from(bytes_read);

        dbg!(bytes_read_so_far);
        Ok(data)
    }

    /// Get cached MusicBrainz data, if available
    fn musicbrainz(&self) -> Option<&DiscId> {
        self.disc().musicbrainz.as_ref()
    }

    /// Rip a single track, returning track info and raw data.
    fn rip(&mut self, track_number: usize) -> io::Result<RippedTrack> {
        let release = self
            .disc()
            .release()
            // TODO handle no releases vs none selected
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No releases found"))?;

        let track_name = release
            .media
            .as_ref()
            .unwrap()
            .first()
            .unwrap()
            .tracks
            .as_ref()
            .unwrap()
            .iter()
            .find(|track| {
                track.number.as_ref().and_then(|number| number.parse().ok()) == Some(track_number)
            })
            .unwrap()
            .title
            .as_ref()
            .unwrap()
            .clone();

        Ok(RippedTrack {
            track_number,
            track_name,
            raw_data: self.read_track(track_number)?,
        })
    }

    /// Rip all tracks, returning a vector of RippedTrack.
    fn rip_all(&mut self) -> io::Result<Vec<RippedTrack>> {
        let release = self
            .disc()
            .release()
            // TODO handle no releases vs none selected
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No releases found"))?;

        let track_numbers: Vec<usize> = release
            .media
            .as_ref()
            .unwrap()
            .first()
            .unwrap()
            .tracks
            .as_ref()
            .unwrap()
            .iter()
            .map(|track| track.number.as_ref().and_then(|n| n.parse().ok()).unwrap())
            .collect();

        track_numbers.iter().map(|&n| self.rip(n)).collect()
    }
}

/// A ripped CD audio track
#[derive(Debug, Clone)]
pub struct RippedTrack {
    pub track_number: usize,
    pub track_name: String,
    pub raw_data: Vec<u8>,
}

pub fn into_wav(pcm: Vec<u8>) -> Vec<u8> {
    // based on https://github.com/Bloomca/rust-cd-da-reader/blob/fd71208262c199dc44d8a012731be298a848ea79/src/lib.rs#L226
    // & https://github.com/Bloomca/rust-cd-da-reader/blob/main/src/utils.rs#L49
    let pcm_data_size = pcm.len();
    let mut wav = Vec::with_capacity(44 + pcm_data_size);
    let pcm_data_size = pcm_data_size as u32;

    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(pcm_data_size + 36).to_le_bytes()); // file size - 8
    wav.extend_from_slice(b"WAVE");

    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes()); // fmt chunk size
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM format
    wav.extend_from_slice(&2u16.to_le_bytes()); // channels
    wav.extend_from_slice(&44100u32.to_le_bytes()); // sample rate
    wav.extend_from_slice(&176400u32.to_le_bytes()); // byte rate
    wav.extend_from_slice(&4u16.to_le_bytes()); // block align
    wav.extend_from_slice(&16u16.to_le_bytes()); // bits per sample

    // data chunk header
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&pcm_data_size.to_le_bytes());

    wav.extend(&pcm);
    wav
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Track {
    pub track_number: u16,
    pub windows_identifier: u32,
    /// Absolute value, including lead-in (150 frames)
    pub starting_frame: u32,
    pub duration_frames: u32,
    /// Relative value, excluding lead-in (2s)
    pub starting_time: CdTime,
    pub duration: CdTime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CdTime {
    pub min: i8,
    pub sec: i8,
    pub frame: i8,
}

impl CdTime {
    pub fn to_frames(&self) -> u32 {
        (((self.min as u32 * 60) + self.sec as u32) * 75) + self.frame as u32
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Disc {
    pub toc: Toc,
    pub musicbrainz: Option<DiscId>,
    /// Selected release index from musicbrainz.releases. Use [`select_release()`]
    /// or [set_release()] to set and [`release()`] to get.
    ///
    /// - None if no selection made yet
    /// - Some(0) if no data present
    /// - Some(0) if first release selected
    /// - Some(n) if specific release selected
    release_index: Option<usize>,
    /// Cached coverart: None if musicbrainz is None
    coverart: Option<Vec<u8>>,
}

impl Disc {
    /// Create a Disc from a given Toc and attempt to identify it on MusicBrainz.
    ///
    /// Will only call MusicBrainz API once to avoid spamming the API
    fn from_toc(toc: Toc) -> Self {
        let discid = toc.musicbrainz_id().to_string();
        let url = format!("https://musicbrainz.org/ws/2/discid/{discid}?inc=recordings&fmt=json");

        let client = reqwest::blocking::Client::new();
        let musicbrainz = try {
            client
                .get(url)
                .header("User-Agent", "splurt/0.1.0")
                .send()
                .ok()?
                .json::<DiscId>()
                .ok()?
        };
        let release_index = match musicbrainz {
            None => Some(0),
            Some(ref mb_data) if mb_data.releases.is_empty() => Some(0),
            Some(ref mb_data) if mb_data.releases.len() == 1 => Some(1),
            _ => None,
        };
        Self {
            toc,
            musicbrainz,
            release_index,
            coverart: None,
        }
    }

    /// Attempt to get the front cover art from the CoverArtArchive.
    ///
    /// - Will return None if no musicbrainz data is available
    /// - Will cache the image to avoid spamming API on repeat calls
    /// - Will not attempt to identify the MusicBrainz ID to avoid spamming API
    pub fn cover_art(&mut self) -> io::Result<&Option<Vec<u8>>> {
        if self.coverart.is_none() {
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
                let image = response.bytes().map_err(io::Error::other)?.to_vec();
                self.coverart = Some(image);
            } else {
                dbg!(response);
            }
        }
        Ok(&self.coverart)
    }

    /// Get the selected release
    pub fn release(&self) -> Option<&musicbrainz::Release> {
        self.musicbrainz.as_ref()?.releases.get(self.release_index?)
    }

    /// Use the release at the given index, or reset selection to None.
    /// Providing an invalid index will make no change.
    ///
    /// Returns a reference to the release set, to allow for validation.
    pub fn set_release(&mut self, index: Option<usize>) -> Option<&Release> {
        let Some(index) = index else {
            self.release_index = None;
            return None;
        };

        match self.musicbrainz.as_ref()?.releases.get(index) {
            None => None,
            Some(release) => {
                self.release_index = Some(index);
                Some(release)
            }
        }
    }

    /// Provides an iterator over the releases to allow for programatic selection.
    ///
    /// Takes a closure which accepts a slice of releases and returns the index of
    /// the release to select.
    pub fn select_release<F>(&mut self, selector: F) -> Option<&Release>
    where
        F: FnOnce(&[musicbrainz::Release]) -> Option<usize>,
    {
        self.set_release(selector(&self.musicbrainz.as_ref()?.releases))
    }
}
