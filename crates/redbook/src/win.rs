#![allow(unsafe_code)]

//! Safe and sane wrappers around windows APIs

// RULES for this file:
// - Use .strict_... for all math functions
// - Panic on failure for any type conversions
// - Use a liberal application of debug_assert

use std::{
    fs::{self, read_dir},
    io,
    path::Path,
    ptr::{null, null_mut},
};

use windows_sys::{
    Win32::{
        Devices::Cdrom::{
            CDROM_READ_TOC_EX, CDROM_TOC, IOCTL_CDROM_READ_TOC_EX, RAW_READ_INFO, TRACK_MODE_TYPE,
        },
        Foundation::HANDLE,
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

use crate::{AudioCdExt, CdTime, FRAME_SIZE, Track};

//(?) https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddcdrm/ne-ntddcdrm-_track_mode_type
pub const TRACK_MODE_CDDA: TRACK_MODE_TYPE = 2;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AudioCd {
    drive: HANDLE,
    /// sorted by position on disc (starting frame)
    tracks: Vec<Track>,
    leadout_starting_frame: u32,
}

impl AudioCd {
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        // Windows already helpfully decodes the TOC for us. Parsing .cda files avoids calling
        // ugly & unsafe ffi functions and wrangling the returned, nested structs.
        let mut tracks: Vec<_> = read_dir(&path)?
            .map(|track| {
                Track::try_from(CdaFile {
                    raw: fs::read(track.unwrap().path()).unwrap(),
                })
                .unwrap()
            })
            .collect();
        tracks.sort_by_key(|track| track.starting_frame);

        // Need to use ffi to raw read data. No API found to "get track audio data".
        let windrive = format!(r"\\.\{}", path.as_ref().display());
        let lpfilename = WinString::from(windrive.as_str());
        let dwdesiredaccess = GENERIC_READ;
        let dwsharemode = FILE_SHARE_READ;
        let dwcreationdisposition = OPEN_EXISTING;
        let drive: HANDLE = unsafe {
            // SAFETY - lpfilename remains valid while raw pointer is in use
            CreateFile2(
                lpfilename.as_pcwstr(),
                dwdesiredaccess,
                dwsharemode,
                dwcreationdisposition,
                null(),
            )
        };
        if drive == INVALID_HANDLE_VALUE {
            // If the function fails, the return value is INVALID_HANDLE_VALUE. To get extended error information, call GetLastError.
            return Err(io::Error::last_os_error());
        };

        let toc_command = CDROM_READ_TOC_EX {
            SessionTrack: 1,
            ..Default::default()
        };
        let mut toc: CDROM_TOC = CDROM_TOC::default();
        let mut bytes_read: u32 = 0;

        let read_toc = unsafe {
            // SAFETY: inline based on
            // https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddcdrm/ni-ntddcdrm-ioctl_cdrom_read_toc_ex
            DeviceIoControl(
                drive,
                IOCTL_CDROM_READ_TOC_EX,
                // points to a buffer of type CDROM_READ_TOC_EX
                &toc_command as *const _ as *const _,
                // indicates the size, in bytes, of the input buffer,
                // which must be >= sizeof(CDROM_READ_TOC_EX).
                size_of_val(&toc_command) as u32,
                // CDROM_READ_TOC_EX does not allow setting `Format`.
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
        }
        debug_assert_eq!(toc.FirstTrack, tracks.first().unwrap().track_number as u8);
        debug_assert_eq!(toc.LastTrack, tracks.last().unwrap().track_number as u8);
        let leadout_address = toc
            .TrackData
            .iter()
            .find(|track| track.TrackNumber == 170)
            .expect("leadout")
            .Address;
        let leadout_starting_frame = u32::from_be_bytes(leadout_address) + 150;
        dbg!(leadout_starting_frame);
        dbg!(toc.FirstTrack);
        dbg!(toc.LastTrack);
        dbg!(toc.Length);
        for track in toc.TrackData {
            if track.TrackNumber != 0 {
                // LeadOut is 170
                dbg!(track.TrackNumber);
                dbg!(track.Address);
            }
        }
        Ok(Self {
            drive,
            tracks,
            leadout_starting_frame,
        })
    }
}

impl AudioCdExt for AudioCd {
    fn track(&self, track_number: usize) -> Option<&Track> {
        self.tracks
            .iter()
            .find(|track| track.track_number == track_number as u16)
    }

    fn leadout(&self) -> u32 {
        self.leadout_starting_frame
    }

    fn toc(&self) -> Result<cdtoc::Toc, cdtoc::TocError> {
        let audio: Vec<_> = self
            .tracks
            .iter()
            .map(|track| track.starting_frame)
            .collect();
        let data = None;
        let leadout = self.leadout();
        cdtoc::Toc::from_parts(audio, data, leadout)
    }

    fn read_chunk(
        &self,
        track: &Track,
        frame_offset: usize,
        frames_to_read: u32,
        buf: &mut [u8],
    ) -> io::Result<u32> {
        let offset = Sector::from_frame(track.starting_frame + frame_offset as u32).offset();
        let read_command = RAW_READ_INFO {
            DiskOffset: offset,
            SectorCount: frames_to_read,
            TrackMode: TRACK_MODE_CDDA,
        };

        let bytes_to_read = frames_to_read * FRAME_SIZE as u32;

        let mut bytes_read: u32 = 0;
        dbg!(offset);

        // SAFETY - inline based on https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddcdrm/ni-ntddcdrm-ioctl_cdrom_raw_read
        let read_chunk = unsafe {
            // SAFETY: Buffer is expected size
            debug_assert_eq!(bytes_to_read, buf.len() as u32);
            // SAFETY: Buffer is exact size for Sector count
            debug_assert_eq!(
                read_command.SectorCount,
                bytes_to_read
                    .div_exact(FRAME_SIZE.try_into().unwrap())
                    .expect("no remainder")
            );

            DeviceIoControl(
                self.drive,
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
impl TryFrom<CdaFile> for Track {
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
        let windows_identifier =
            u32::from_le_bytes([data[0x18], data[0x19], data[0x1A], data[0x1B]]);
        let range_offset_frames =
            u32::from_le_bytes([data[0x1C], data[0x1D], data[0x1E], data[0x1F]]);
        // For inexplicable, probably historical, reasons Windows stores the relative frame in cda
        let starting_frame = range_offset_frames + 150;
        let duration_frames = u32::from_le_bytes([data[0x20], data[0x21], data[0x22], data[0x23]]);

        // For inexplicable, probably historical, reasons Windows stores the absolute time in cda
        let starting_time = CdTime {
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
        let duration = CdTime {
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

        debug_assert_eq!(starting_frame, starting_time.to_frames() + 150);
        debug_assert_eq!(duration_frames, duration.to_frames());

        Ok(Track {
            track_number,
            windows_identifier,
            starting_frame,
            duration_frames,
            starting_time,
            duration,
        })
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
    pub fn from_frame(frame: u32) -> Self {
        Self(frame as i64 - 150)
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
    pub unsafe fn as_pcwstr(&self) -> PCWSTR {
        self.words.as_ptr()
    }
}
