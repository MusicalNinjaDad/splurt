#![feature(exact_div)]
#![feature(exact_size_is_empty)]
#![feature(iter_array_chunks)]
#![feature(iterator_try_collect)]
#![feature(try_blocks)]
// Unsafe restricted to dedicated wrapper modules
#![deny(unsafe_code)]
#![forbid(unsafe_op_in_unsafe_fn)]
#![forbid(unsafe_attr_outside_unsafe)]

//! CDDA CD digital audio as per RedBook (IEC 60908:1999)
//!
//! Frame IDs are always *absolute* and *include* the lead-in (150 frames)

pub mod hex;
pub mod musicbrainz;
pub mod win;

pub use win::AudioCd;
use windows_sys::Win32::Devices::Cdrom::TRACK_DATA;

use std::{
    convert::TryFrom,
    io,
    ops::{Add, Rem, Sub},
    time::Duration,
};

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

pub const LEADIN: Frame = Frame(150);

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
        let track_size = track.duration_frames.as_usize().strict_mul(FRAME_SIZE);
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
            .as_usize()
            .strict_rem(MAX_CHUNK_FRAMES);

        let bytes_read = self.read_chunk(track, frame_offset, frames_to_read as u32, last_buf)?;
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
    pub toc_entry: TocEntry,
    pub duration_frames: Frame,
    pub windows_identifier: Option<u32>,
}

/// Entry in a CD TOC (Table of Contents)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TocEntry {
    pub track: u8,
    /// Absolute value, including lead-in (150 frames)
    pub start: Frame,
}

/// CD audio frame (1/75 sec). Basic unit of time for CD audio
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Frame(usize);

impl Frame {
    pub fn new(frames: usize) -> Self {
        Self(frames)
    }

    pub fn as_usize(self) -> usize {
        self.0
    }

    pub fn relative_to_leadin(self) -> Self {
        self - LEADIN
    }
}

impl From<&TRACK_DATA> for TocEntry {
    // TODO make fallible TryFrom
    fn from(track_data: &TRACK_DATA) -> Self {
        let relative = u32::from_be_bytes(track_data.Address);
        let start = Frame::new(relative as usize) + LEADIN;
        let track = track_data.TrackNumber;
        Self { track, start }
    }
}

impl From<Msf> for Frame {
    fn from(msf: Msf) -> Self {
        Self((((msf.min as usize * 60) + msf.sec as usize) * 75) + msf.frame as usize)
    }
}

impl From<Duration> for Frame {
    fn from(duration: Duration) -> Self {
        Msf::from(duration).into()
    }
}

impl From<Frame> for Duration {
    fn from(frames: Frame) -> Self {
        Msf::from(frames).into()
    }
}

impl<N> Add<N> for Frame
where
    usize: Add<N, Output = usize>,
{
    type Output = Self;

    fn add(self, rhs: N) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl Add<Frame> for Frame {
    type Output = Self;

    fn add(self, rhs: Frame) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub<Frame> for Frame {
    type Output = Self;

    fn sub(self, rhs: Frame) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl<N> Sub<N> for Frame
where
    usize: Sub<N, Output = usize>,
{
    type Output = Self;

    fn sub(self, rhs: N) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl<N> Rem<N> for Frame
where
    usize: Rem<N, Output = usize>,
{
    type Output = Self;

    fn rem(self, rhs: N) -> Self::Output {
        Self(self.0 % rhs)
    }
}

impl PartialEq<Msf> for Frame {
    fn eq(&self, msf: &Msf) -> bool {
        *self == Self::from(*msf)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// CD audio duration in min:sec/frames (75 frames/sec)
pub struct Msf {
    min: i8,
    sec: i8,
    frame: i8,
}

impl Msf {
    pub fn new(min: i8, sec: i8, frame: i8) -> Self {
        Self { min, sec, frame }
    }

    pub fn relative_to_leadin(self) -> Self {
        self - LEADIN
    }
}

impl Sub<Frame> for Msf {
    type Output = Self;

    fn sub(self, rhs: Frame) -> Self::Output {
        (Frame::from(self) - rhs).into()
    }
}

impl From<Duration> for Msf {
    fn from(duration: Duration) -> Self {
        let ms = duration.as_millis();
        let secs = ms / 1000;
        let min = secs / 60;
        let secs = secs % 60;
        let frames = (ms % 1000) * 75 / 1000;
        Self {
            min: min as i8,
            sec: secs as i8,
            frame: frames as i8,
        }
    }
}

impl From<Msf> for Duration {
    fn from(msf: Msf) -> Self {
        let secs = (msf.min * 60) + msf.sec;
        let nanos = msf.frame as u32 * 75 / 1_000_000_000;
        Self::new(secs as u64, nanos)
    }
}

impl From<Frame> for Msf {
    fn from(frames: Frame) -> Self {
        let frames = frames.as_usize();
        let secs = frames / 75;
        let min = secs / 60;
        let secs = secs % 60;
        let frames = frames % 75;
        Self {
            min: min as i8,
            sec: secs as i8,
            frame: frames as i8,
        }
    }
}

impl PartialEq<Frame> for Msf {
    fn eq(&self, frame: &Frame) -> bool {
        Frame::from(*self) == *frame
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

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn leadin_conversion() {
        let dur = Duration::from_secs(2);
        let msf = Msf {
            min: 0,
            sec: 2,
            frame: 0,
        };
        let frames = LEADIN;

        assert_eq!(Msf::from(dur), msf);
        assert_eq!(Msf::from(frames), msf);
        assert_eq!(Frame::from(msf), frames);
        assert_eq!(Frame::from(dur), frames);
        assert_eq!(Duration::from(msf), dur);
        assert_eq!(Duration::from(frames), dur);
    }

    #[test]
    fn leadin_compensation() {
        let starting_frames = Frame::new(183);
        let starting_time = Msf::new(0, 2, 33);
        assert_eq!(starting_frames.relative_to_leadin(), Frame::new(33));
        assert_eq!(starting_time.relative_to_leadin(), Msf::new(0, 0, 33));
    }
}
