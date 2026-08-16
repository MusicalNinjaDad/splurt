#![feature(exact_div)]
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

    fn musicbrainz(&self) -> io::Result<DiscId> {
        let brainz = self.toc().unwrap().musicbrainz_url();
        dbg!(brainz);

        let discid = self.toc().unwrap().musicbrainz_id().to_string();
        dbg!(&discid);
        let url = format!("https://musicbrainz.org/ws/2/discid/{discid}?inc=recordings&fmt=json");
        dbg!(&url);
        let client = reqwest::blocking::Client::new();
        let response = client
            .get(url)
            .header("User-Agent", "splurt/0.1.0")
            .send()
            .map_err(io::Error::other)?;
        dbg!(&response);
        let details = response.json().map_err(io::Error::other)?;
        Ok(details)
    }
}

pub fn rip(cd: AudioCd, track_number: usize) -> io::Result<(String, Vec<u8>)> {
    let mb_data = cd.musicbrainz()?;
    // dbg!(&mb_data);
    let track_name = mb_data
        .releases
        .first()
        .unwrap()
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
        .unwrap();
    dbg!(track_name);

    Ok((track_name.clone(), cd.read_track(track_number)?))
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
