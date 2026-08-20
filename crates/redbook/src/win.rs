//! Safe and sane wrappers around windows APIs

// RULES for this file:
// - Use .strict_... for all math functions
// - Panic on failure for any type conversions
// - Use a liberal application of debug_assert

use std::{
    fmt::Debug,
    fs::{self, read_dir},
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    ptr::{null, null_mut},
};

use cdtoc::{Toc, TocError};
use windows_sys::{
    Win32::{
        Devices::Cdrom::{
            CDROM_READ_TOC_EX, CDROM_TOC, IOCTL_CDROM_READ_TOC_EX, RAW_READ_INFO, TRACK_MODE_TYPE,
        },
        Foundation::{CloseHandle, HANDLE},
        Storage::FileSystem::CreateFile2,
        System::IO::DeviceIoControl,
    },
    core::PCWSTR,
};

use windows_sys::Win32::{
    Devices::Cdrom::IOCTL_CDROM_RAW_READ,
    Foundation::{GENERIC_READ, INVALID_HANDLE_VALUE},
    Storage::FileSystem::{FILE_SHARE_READ, OPEN_EXISTING},
};

use crate::disc::Disc;
use crate::{AudioCdExt, FRAME_SIZE, Frame, LEADIN, Msf, Track};
use crate::{TocEntry, hex::hex_dump};

//(?) https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddcdrm/ne-ntddcdrm-_track_mode_type
pub const TRACK_MODE_CDDA: TRACK_MODE_TYPE = 2;

const TOC_SIZE: usize = size_of::<CDROM_TOC>();

/// A CdDrive with opened read-only [`HANDLE`] and [`CDROM_TOC`]
///
/// # SAFETY
/// - CdDrive cannot be `Clone` to avoid duplicate handles
pub struct CdDrive {
    path: PathBuf,
    handle: HANDLE,
    toc: CDROM_TOC,
}

/// # SAFETY
/// - See safety restrictions on [Self::handle]
/// - Not Sync as we have not enabled overlapped I/O or any internal sync mechanism
#[allow(unsafe_code)]
unsafe impl Send for CdDrive {}

impl Debug for CdDrive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CdDrive")
            .field("path", &self.path)
            .field("handle", &self.handle)
            .field("toc", &self.toc_as_raw_bytes())
            .finish()
    }
}

impl PartialEq for CdDrive {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
            && self.handle == other.handle
            && self.toc_as_raw_bytes() == other.toc_as_raw_bytes()
    }
}

impl Eq for CdDrive {}

impl CdDrive {
    /// A safe wrapper around the ffi calls needed to obtain a handle for the raw
    /// drive at `path` and obtain the TOC as provided by the relevant windows system
    /// call `DeviceIoControl(..,IOCTL_CDROM_READ_TOC_EX,..)`
    ///
    /// - The returned [`CdDrive`] provides methods to access the handle and TOC.
    /// - The handle has minimal (shared read only) access rights and will be closed
    ///   when the [`CdDrive`] is dropped. Consider using [exit_safely] to ensure that
    ///   this occurs in your binary even on error.
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let path: PathBuf = PathBuf::from(path.as_ref());
        let windrive = format!(r"\\.\{}", path.display());
        let lpfilename = WinString::from(windrive.as_str());
        let dwdesiredaccess = GENERIC_READ;
        let dwsharemode = FILE_SHARE_READ;
        let dwcreationdisposition = OPEN_EXISTING;
        #[allow(unsafe_code)]
        let handle: HANDLE = unsafe {
            // SAFETY: `lpfilename` is a local variable that remains
            // valid for the duration of this synchronous FFI call.
            CreateFile2(
                lpfilename.as_pcwstr(),
                dwdesiredaccess,
                dwsharemode,
                dwcreationdisposition,
                null(),
            )
        };
        // If the function fails, the return value is INVALID_HANDLE_VALUE.
        // To get extended error information, call GetLastError.
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        };

        let toc_command = CDROM_READ_TOC_EX {
            SessionTrack: 1,
            ..Default::default()
        };

        let mut toc = CDROM_TOC::default();
        let mut bytes_read: u32 = 0;

        #[allow(unsafe_code)]
        let read_toc = unsafe {
            // SAFETY: inline based on
            // https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddcdrm/ni-ntddcdrm-ioctl_cdrom_read_toc_ex
            DeviceIoControl(
                // valid handle - we have just created it
                handle,
                IOCTL_CDROM_READ_TOC_EX,
                // points to a buffer of type CDROM_READ_TOC_EX
                &toc_command as *const _ as *const _,
                // indicates the size, in bytes, of the input buffer,
                // which must be >= sizeof(CDROM_READ_TOC_EX).
                size_of_val(&toc_command) as u32,
                // CDROM_READ_TOC_EX does not allow setting `Format` but
                // `CDROM_READ_TOC_EX_FORMAT_TOC` is `0` (default) whereby
                // The output data is reported in a CDROM_TOC structure.
                &mut toc as *mut _ as *mut _,
                size_of_val(&toc) as u32,
                &mut bytes_read as *mut _,
                null_mut(),
            )
        };
        if read_toc == 0 {
            dbg!(bytes_read);
            return Err(io::Error::last_os_error());
        };
        assert!(bytes_read <= TOC_SIZE as u32);
        Ok(Self { path, handle, toc })
    }

    /// The path of the drive
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Obtain a reference to the [`HANDLE`] for the drive.
    ///
    /// # SAFETY
    /// - [`CdDrive`] is marked as [`Send`]. Callers must ensure that the handle is not
    ///   used to enable concurrent access to the drive ("processes and threads that share
    ///   the same file must synchronize their access").
    ///   See: https://learn.microsoft.com/en-us/windows/win32/fileio/file-handles
    #[allow(unsafe_code)]
    pub unsafe fn handle(&self) -> &HANDLE {
        &self.handle
    }

    /// Obtain an array of raw bytes representing the [`CDROM_TOC`]
    pub fn toc_as_raw_bytes(&self) -> &[u8] {
        #[allow(unsafe_code)]
        unsafe {
            // SAFETY - check stored value is the expected size
            assert_eq!(size_of_val(&self.toc), TOC_SIZE);
            std::slice::from_raw_parts(&self.toc as *const _ as *const _, TOC_SIZE)
        }
    }

    /// Obtain a hex representation of the raw bytes representing the [`CDROM_TOC`]
    pub fn toc_as_hex(&self) -> String {
        hex_dump(self.toc_as_raw_bytes())
    }

    /// Obtain a reference to the TOC as a [`CDROM_TOC`]
    pub fn toc(&self) -> &CDROM_TOC {
        &self.toc
    }
}

impl Drop for CdDrive {
    fn drop(&mut self) {
        #[allow(unsafe_code)]
        unsafe {
            // SAFETY: handle
            // - was opened and validated in `open()`
            // - has not been closed (no such methods provided on Self)
            // - has not been externally mutated (no such methods provided on Self)
            CloseHandle(self.handle as *mut _);
        }
        // Not checking for success: cannot meaningfully handle CloseHandle failure during drop
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct AudioCd {
    drive: CdDrive,
    /// sorted by position on disc (starting frame)
    tracks: Vec<Track<'static>>,
    leadout_starting_frame: u32,
    disc: Disc,
}

impl AudioCd {
    /// Opens drive, reads CD, gets data from musicbrainz & coverart if possible
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        // Windows already helpfully decodes the TOC for us. Parsing .cda files pre-calculates the
        // durations and gives us a comparison to validate the raw TOC against.
        let mut tracks: Vec<_> = read_dir(&path)?
            .map(|track| {
                Track::try_from(CdaFile {
                    raw: fs::read(track?.path())?,
                })
                .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))
            })
            .try_collect()?;
        tracks.sort_by_key(|track| track.toc_entry.start);

        let drive = CdDrive::open(path)?;

        let wintoc = drive.toc();
        (wintoc.FirstTrack
            == tracks
                .first()
                .ok_or(io::Error::new(ErrorKind::NotFound, "no .cda files found"))?
                .toc_entry
                .track as u8)
            .ok_or(io::Error::new(
                ErrorKind::InvalidData,
                "Mismatch between TOC and cda files: different first track number",
            ))?;
        (wintoc.LastTrack
            == tracks
                .last()
                .ok_or(io::Error::new(ErrorKind::NotFound, "no .cda files found"))?
                .toc_entry
                .track as u8)
            .ok_or(io::Error::new(
                ErrorKind::InvalidData,
                "Mismatch between TOC and cda files: different last track number",
            ))?;

        for track in tracks.iter() {
            let track_number = track.toc_entry.track as usize;
            let data = wintoc
                .TrackData
                .get(track_number - 1)
                .ok_or(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("track {track_number} missing in TOC"),
                ))?;
            (TocEntry::from(data) == track.toc_entry).ok_or(io::Error::new(
                ErrorKind::InvalidData,
                format!("Mismatch between TOC and cda files for track {track_number}"),
            ))?;
        }

        let toc = wintoc
            .to_toc()
            .map_err(|error| io::Error::new(ErrorKind::InvalidData, error))?;

        let leadout = toc.leadout();

        let mut disc = Disc::new(toc, tracks.clone(), Frame::new(leadout as usize))?;
        let _ = disc.update_musicbrainz();
        let _ = disc.update_cover_art();

        Ok(Self {
            drive,
            tracks,
            leadout_starting_frame: leadout,
            disc,
        })
    }
}

impl AudioCdExt for AudioCd {
    fn track(&self, track_number: usize) -> Option<&Track<'_>> {
        self.tracks
            .iter()
            .find(|track| track.toc_entry.track == track_number as u8)
    }

    fn leadout(&self) -> u32 {
        self.leadout_starting_frame
    }

    fn toc(&self) -> Result<cdtoc::Toc, cdtoc::TocError> {
        let audio: Vec<_> = self
            .tracks
            .iter()
            .map(|track| track.toc_entry.start.as_usize() as u32)
            .collect();
        let data = None;
        let leadout = self.leadout();
        cdtoc::Toc::from_parts(audio, data, leadout)
    }

    fn disc(&self) -> &Disc {
        &self.disc
    }

    fn disc_mut(&mut self) -> &mut Disc {
        &mut self.disc
    }

    fn tracks(&self) -> impl Iterator<Item = &Track<'_>> {
        self.tracks.iter()
    }

    fn read_chunk(
        &self,
        track: &Track,
        frame_offset: usize,
        frames_to_read: u32,
        buf: &mut [u8],
    ) -> io::Result<u32> {
        let offset = Sector::from_frame(track.toc_entry.start + frame_offset).offset();
        let read_command = RAW_READ_INFO {
            DiskOffset: offset,
            SectorCount: frames_to_read,
            TrackMode: TRACK_MODE_CDDA,
        };

        let bytes_to_read = frames_to_read * FRAME_SIZE as u32;

        let mut bytes_read: u32 = 0;
        dbg!(offset);

        #[allow(unsafe_code)]
        // SAFETY - inline based on https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddcdrm/ni-ntddcdrm-ioctl_cdrom_raw_read
        let read_chunk = unsafe {
            // SAFETY check: Buffer is expected size.
            // Runtime check as `buf` is provided by caller
            (bytes_to_read == buf.len() as u32)
                .ok_or_else(|| io::Error::new(
                    ErrorKind::InvalidInput,
                    format!("buffer incorrectly sized for track data. Require {bytes_to_read} bytes, buffer is {len} bytes", len = buf.len())
                )
            )?;

            // SAFETY check: Buffer is exact size for Sector count.
            // Debug check as we generated SectorCount and have validated bytes_to_read above.
            debug_assert_eq!(
                read_command.SectorCount,
                bytes_to_read
                    .div_exact(FRAME_SIZE.try_into().unwrap())
                    .expect("no remainder")
            );

            DeviceIoControl(
                *self.drive.handle(),
                IOCTL_CDROM_RAW_READ,
                // If the IOCTL is from user mode, Irp->AssociatedIrp.SystemBuffer contains a RAW_READ_INFO
                // structure that specifies the starting disk offset, the sector count, and the track mode
                // (XA or CDDA) for the read.
                &read_command as *const _ as *const _,
                // Parameters.DeviceIoControl.InputBufferLength specifies the size, in bytes, of the
                // structure, which must be >= sizeof(RAW_READ_INFO)
                size_of_val(&read_command) as u32,
                // Cannot reallocate without risking invalidating pointer. We create frame with capacity
                // equal to read_command.SectorCount * Sectorsize.
                buf as *mut _ as *mut _,
                // Parameters.DeviceIoControl.OutputBufferLength
                // specifies the size of the buffer to be read, which must be >= sizeof(SectorCount * RAW_SECTOR_SIZE)
                bytes_to_read,
                &mut bytes_read as *mut _,
                null_mut(),
            )
        };
        if read_chunk == 0 {
            dbg!(bytes_read);
            return Err(io::Error::last_os_error());
        }
        debug_assert_eq!(
            bytes_read, bytes_to_read,
            "intended to read {bytes_to_read} bytes from offset {offset} but only got {bytes_read}"
        );
        Ok(bytes_read)
    }
}

/// A windows .cda file detailling CD TOC info for a given track
pub struct CdaFile {
    raw: Vec<u8>,
}

/// Parsing based on https://en.wikipedia.org/wiki/.cda_file
impl TryFrom<CdaFile> for Track<'static> {
    type Error = io::Error;

    fn try_from(cda: CdaFile) -> Result<Self, Self::Error> {
        let data = cda.raw;
        const MIN_LEN: usize = 44;
        if data.len() < MIN_LEN {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "CDA file too short: expected at least {} bytes, got {}",
                    MIN_LEN,
                    data.len()
                ),
            ));
        }

        // Validate RIFF header
        if &data[0..4] != b"RIFF" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing or invalid RIFF header",
            ));
        }

        // Validate chunk size (always 36)
        let chunk_size = u32::from_le_bytes([data[4], data[5], data[6], data[7]]);
        if chunk_size != 36 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid chunk size: expected 36, got {}", chunk_size),
            ));
        }

        // Validate CDDA identifier
        if &data[8..12] != b"CDDA" {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing or invalid CDDA identifier",
            ));
        }

        // Validate fmt chunk identifier
        if &data[12..16] != b"fmt " {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Missing or invalid fmt chunk identifier",
            ));
        }

        // Validate fmt chunk size (always 24)
        let fmt_chunk_size = u32::from_le_bytes([data[16], data[17], data[18], data[19]]);
        if fmt_chunk_size != 24 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Invalid fmt chunk size: expected 24, got {}",
                    fmt_chunk_size
                ),
            ));
        }

        // Parse version (always 1)
        let version = u16::from_le_bytes([data[0x14], data[0x15]]);
        if version != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Invalid version: expected 1, got {}", version),
            ));
        }

        let track_number = u16::from_le_bytes([data[0x16], data[0x17]]);
        let windows_identifier = Some(u32::from_le_bytes([
            data[0x18], data[0x19], data[0x1A], data[0x1B],
        ]));
        let range_offset_frames =
            u32::from_le_bytes([data[0x1C], data[0x1D], data[0x1E], data[0x1F]]);
        // For inexplicable, probably historical, reasons Windows stores the relative frame in cda
        let starting_frame = Frame(range_offset_frames as usize) + LEADIN;
        let duration_frames = u32::from_le_bytes([data[0x20], data[0x21], data[0x22], data[0x23]]);
        let duration_frames = Frame(duration_frames as usize);

        // For inexplicable, probably historical, reasons Windows stores the absolute time in cda
        let starting_time = Msf {
            frame: data[0x24] as i8,
            sec: data[0x25] as i8 - 2,
            min: data[0x26] as i8,
        };

        // Validate null byte after range position
        if data[0x27] != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Expected null byte after range position",
            ));
        }

        // Parse duration time
        let duration = Msf {
            frame: data[0x28] as i8,
            sec: data[0x29] as i8,
            min: data[0x2A] as i8,
        };

        // Validate null byte after duration
        if data[0x2B] != 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Expected null byte after duration",
            ));
        }

        // For inexplicable, probably historical, reasons Windows stores the
        // *relative* frame and *absolute* time in cda
        debug_assert_eq!(starting_frame.relative_to_leadin(), starting_time);
        debug_assert_eq!(duration_frames, duration);

        let toc_entry = TocEntry {
            track: track_number as u8,
            start: starting_frame,
        };

        Ok(Track {
            toc_entry,
            windows_identifier,
            duration_frames,
            ..Default::default()
        })
    }
}

/// Manipulation of [`CDROM_TOC`]
pub trait CdromTocExt {
    /// Parse raw bytes as a CDROM_TOC structure
    ///
    /// # Safety
    /// The caller must ensure `bytes` is exactly the size of CDROM_TOC and properly aligned.
    #[allow(unsafe_code)]
    unsafe fn from_raw_bytes(bytes: Vec<u8>) -> CDROM_TOC;

    fn to_toc(&self) -> Result<Toc, TocError>;

    fn iter_audio(&self) -> impl Iterator<Item = TocEntry>;

    /// The absolute start of the lead out
    fn leadout(&self) -> Result<Frame, TocError>;
}

impl CdromTocExt for CDROM_TOC {
    #[allow(unsafe_code)]
    unsafe fn from_raw_bytes(bytes: Vec<u8>) -> CDROM_TOC {
        #[allow(unsafe_code)]
        unsafe {
            *(bytes.as_ptr() as *const _)
        }
    }

    fn to_toc(&self) -> Result<Toc, TocError> {
        let audio = self
            .iter_audio()
            .map(|entry| entry.start.as_usize() as u32)
            .collect();
        let leadout = self.leadout()?.as_usize() as u32;
        Toc::from_parts(audio, None, leadout)
    }

    fn iter_audio(&self) -> impl Iterator<Item = TocEntry> {
        self.TrackData
            .iter()
            .filter(|track| (1..0xA0).contains(&track.TrackNumber))
            .map(TocEntry::from)
    }

    fn leadout(&self) -> Result<Frame, TocError> {
        self.TrackData
            .iter()
            .find(|track| track.TrackNumber == 170)
            .map(TocEntry::from)
            .map(|entry| entry.start)
            .ok_or(TocError::SectorOrder)
    }
}

/// A pseudo-sector on an AudioCd
///
/// Windows DeviceIoControl wants offsets which pretend a [FRAME_SIZE]-byte frame is a 2048-byte
/// sector.
///
/// Internally stores the relative frame (excluding 150 lead-in frames)
pub struct Sector(i64);

impl Sector {
    /// Construct from an absolute frame number (including lead-in)
    pub fn from_frame(frame: Frame) -> Self {
        Self(frame.relative_to_leadin().as_usize() as i64)
    }

    /// For passing to `DeviceIoControl(..,IOCTL_CDROM_RAW_READ,..)`
    ///
    /// - Pretends that each frame is a 2048-byte sector.
    /// - Returned offset is relative to start of audio data
    pub fn offset(&self) -> i64 {
        self.0 * 2048
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A somewhat sane way of dealing with `PWSTR/PCWSTR`: A pointer to a null terminated string
/// consisting of 'wide chars' (u16), encoded using UTF-16.
///
/// Construct via `WinString::from(&str)`
pub struct WinString {
    words: Vec<u16>,
}

impl From<&str> for WinString {
    fn from(utf8: &str) -> Self {
        // see https://kennykerr.ca/rust-getting-started/string-tutorial.html
        let words = utf8.encode_utf16().chain(Some(0)).collect();
        Self { words }
    }
}

impl WinString {
    /// Create a `PCWSTR` - note this is a raw pointer.
    ///
    /// # SAFETY
    /// You must ensure that the returned `PCWSTR` is not used after self is dropped.
    /// It is recommended to call this directly in the call to a WinAPI unsafe function,
    /// see [AudioCd::new()] for an example
    #[allow(unsafe_code)]
    pub unsafe fn as_pcwstr(&self) -> PCWSTR {
        self.words.as_ptr()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hex::hex_to_bytes;

    fn load_toc() -> CDROM_TOC {
        let toc_dump = hex_to_bytes(include_str!(
            "../tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/CDROM_TOC.hex"
        ))
        .unwrap();
        #[allow(unsafe_code)]
        let toc = unsafe { CDROM_TOC::from_raw_bytes(toc_dump) };
        assert_eq!(toc.FirstTrack, 1);
        assert_eq!(toc.LastTrack, 11);
        toc
    }

    #[test]
    fn leadout() {
        let toc = load_toc();
        let leadout = toc.leadout().unwrap();
        assert_eq!(leadout, Frame::from(Msf::new(0x34, 0x05, 0x1c)))
    }

    #[test]
    fn audio() {
        let toc = load_toc();
        let audio_tracks: Vec<TocEntry> = toc.iter_audio().collect();
        let definitely_maybe = [
            TocEntry {
                track: 1,
                start: Frame::from(Msf::new(0x00, 0x02, 0x21)),
            },
            TocEntry {
                track: 2,
                start: Frame::from(Msf::new(0x05, 0x19, 0x32)),
            },
            TocEntry {
                track: 3,
                start: Frame::from(Msf::new(0x0A, 0x22, 0x0D)),
            },
            TocEntry {
                track: 4,
                start: Frame::from(Msf::new(0x0F, 0x0B, 0x00)),
            },
            TocEntry {
                track: 5,
                start: Frame::from(Msf::new(0x13, 0x27, 0x44)),
            },
            TocEntry {
                track: 6,
                start: Frame::from(Msf::new(0x19, 0x38, 0x41)),
            },
            TocEntry {
                track: 7,
                start: Frame::from(Msf::new(0x1E, 0x28, 0x2D)),
            },
            TocEntry {
                track: 8,
                start: Frame::from(Msf::new(0x22, 0x3A, 0x21)),
            },
            TocEntry {
                track: 9,
                start: Frame::from(Msf::new(0x27, 0x2F, 0x3A)),
            },
            TocEntry {
                track: 10,
                start: Frame::from(Msf::new(0x2A, 0x14, 0x08)),
            },
            TocEntry {
                track: 11,
                start: Frame::from(Msf::new(0x30, 0x34, 0x3F)),
            },
        ];
        assert_eq!(audio_tracks.as_slice(), &definitely_maybe);
    }
}
