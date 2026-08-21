#![feature(exact_div)]
#![feature(exact_size_is_empty)]
#![feature(iter_array_chunks)]
#![feature(iterator_try_collect)]
#![feature(negative_impls)]
#![feature(path_absolute_method)]
#![feature(try_blocks)]
// Unsafe restricted to dedicated wrapper modules
#![deny(unsafe_code)]
#![forbid(unsafe_op_in_unsafe_fn)]
#![forbid(unsafe_attr_outside_unsafe)]

//! CDDA CD digital audio as per RedBook (IEC 60908:1999)
//!
//! Frame IDs are always *absolute* and *include* the lead-in (150 frames)

pub mod disc;
pub mod hex;
pub mod musicbrainz;
pub mod tagging;
pub mod win;

use tracing::trace;

pub use disc::Disc;
use flacenc::{bitsink::MemSink, component::BitRepr, error::Verify};
pub use win::AudioCd;
use windows_sys::Win32::Devices::Cdrom::TRACK_DATA;

use std::{
    convert::TryFrom,
    io,
    ops::{Add, Rem, Sub},
    sync::Arc,
    time::Duration,
};

use musicbrainz::Discid;

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

    /// Return a reference to the cached Disc data
    fn disc(&self) -> &Arc<crate::Disc>;

    /// Read a full track, returning the raw data as a `Vec` of bytes.
    fn read_track(&self, track_number: usize) -> io::Result<Vec<u8>> {
        let track = self.disc().track(track_number).unwrap();
        tracing::info!(track_number = track.track_number(), "read_track");
        let track_size = track.duration_frames.as_usize().strict_mul(FRAME_SIZE);
        debug_assert!(track_size > 0);

        // Vec needs to be initialised to split into chunks. Performance cost insignificant vs IO.
        let mut data = vec![0_u8; track_size];
        tracing::trace!(data_len = data.len());

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

            let bytes_read = self.read_chunk(&track, frame_offset, frames_to_read, buf)?;
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

        let bytes_read = self.read_chunk(&track, frame_offset, frames_to_read as u32, last_buf)?;
        bytes_read_so_far += i64::from(bytes_read);

        tracing::trace!(bytes_read_so_far);
        Ok(data)
    }

    /// Get cached MusicBrainz data, if available
    fn musicbrainz(&self) -> Option<&Discid> {
        self.disc().musicbrainz()
    }

    /// Rip a single track, returning track info and raw data.
    fn rip(&self, track_number: usize) -> io::Result<RippedTrack> {
        let raw_data = self.read_track(track_number)?;
        Ok(RippedTrack {
            track_number,
            raw_data,
        })
    }
}

pub trait AudioCdExtMut {
    /// Return a mutable reference to the cached Disc data. Warning - as disc
    /// is backed by an Arc this will clone the internal data if other references
    /// currently exist. See [Arc::make_mut] for details on the underlying mechanism.
    fn disc_mut(&mut self) -> &mut crate::disc::Disc;
}

/// A small wrapper with the raw data for a track and the track_number
#[derive(Debug, Clone)]
pub struct RippedTrack {
    pub track_number: usize,
    pub raw_data: Vec<u8>,
}

impl RippedTrack {
    pub fn to_flac(&self) -> MemSink<u8> {
        let (channels, bits_per_sample, sample_rate) = (2, 16, 44100);
        let config = flacenc::config::Encoder::default()
            .into_verified()
            .expect("Config data error.");
        let samples: Vec<_> = self
            .raw_data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as i32)
            .collect();
        let source = flacenc::source::MemSource::from_samples(
            &samples,
            channels,
            bits_per_sample,
            sample_rate,
        );
        let flac_stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
            .expect("Encode failed.");
        let mut sink = flacenc::bitsink::ByteSink::new();
        flac_stream.write(&mut sink).unwrap();
        sink
    }

    pub fn to_wav(&self) -> Vec<u8> {
        let pcm = &self.raw_data;

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

        wav.extend(pcm);
        wav
    }
}
#[derive(Debug, Clone, PartialEq, Default)]
/// Cheap to clone - all fields are `Copy` except ripped data which uses cheap-to-clone [`Bytes`]
/// Borrow checker ensures validity of metadata for lifetime `<'meta>`
/// Usually constructed as `<'static>` then `clone`d when referencing metadata
pub struct Track<'meta> {
    pub toc_entry: TocEntry,
    pub duration_frames: Frame,
    pub windows_identifier: Option<u32>,
    meta: Option<&'meta musicbrainz::Track>,
}

impl<'meta> Track<'meta> {
    pub fn track_number(&self) -> u8 {
        self.toc_entry.track
    }

    pub fn title(&self) -> Option<String> {
        self.meta.map(|track| track.title.clone())
    }

    /// Returns the most likely representation on the track listing, as we expect it was
    /// written on the back of the CD. The only adjusments we makeare to ensure that numerical
    /// track numbers are always 2 digits long, in order to allow alphabetical sorting to work.
    ///
    /// E.g. "05 Columbia", "A1 Speak to Me"
    ///
    /// This will use the text representation for track_number from musicbrainz if available,
    /// falling back to the two digit track number.
    pub fn filename(&self) -> String {
        let track_num = self
            .meta()
            .map(|trk| {
                let trk_num = trk.number.clone();
                match trk_num.len() {
                    1 if trk_num.parse::<usize>().is_ok() => format!("0{trk_num}"),
                    _ => trk_num,
                }
            })
            .unwrap_or_else(|| format!("{:02}", self.toc_entry.track));
        [track_num, self.title().unwrap_or_default()].join(" ")
    }

    pub fn meta(&self) -> Option<&'meta musicbrainz::Track> {
        self.meta
    }
}

/// Entry in a CD TOC (Table of Contents)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TocEntry {
    pub track: u8,
    /// Absolute value, including lead-in (150 frames)
    pub start: Frame,
}

/// CD audio frame (1/75 sec). Basic unit of time for CD audio
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
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
        trace!(
            target: "frame_conversion",
            min = msf.min,
            sec = msf.sec,
            frame = msf.frame,
            "Frame::from(Msf)"
        );
        Self((((msf.min as usize * 60) + msf.sec as usize) * 75) + msf.frame as usize)
    }
}

impl From<Duration> for Frame {
    fn from(duration: Duration) -> Self {
        trace!(
            target: "frame_conversion",
            secs = duration.as_secs(),
            "Frame::from(Duration)"
        );
        Msf::from(duration).into()
    }
}

impl From<Frame> for Duration {
    fn from(frames: Frame) -> Self {
        trace!(
            target: "frame_conversion",
            frames = frames.as_usize(),
            "Duration::from(Frame)"
        );
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
        trace!(
            target: "frame_conversion",
            secs = duration.as_secs(),
            "Msf::from(Duration)"
        );
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
        trace!(
            target: "frame_conversion",
            frames = frames.as_usize(),
            "Msf::from(Frame)"
        );
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
