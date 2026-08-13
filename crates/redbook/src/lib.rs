#![feature(dirfd)]

//! CDDA CD digital audio as per RedBook (IEC 60908:1999)

use std::{
    fs::{self, Dir, read_dir}, io, path::PathBuf, ptr::{null, null_mut},
};

use std::convert::TryFrom;

pub fn read_toc(drive: &str) -> io::Result<()> {
    let drive: PathBuf = drive.into();
    let dir = Dir::open(&drive)?;
    dbg!(&dir);
    let metadata = dir.metadata()?;
    dbg!(&metadata);
    for track in read_dir(&drive)? {
        dbg!(&track);
        let cda = Cda::try_from(fs::read(track?.path())?)?;
        dbg!(&cda);
    }
    let windrive = format!(r"\\.\{}", drive.display());
    let lpfilename = WinString::from(windrive.as_str());
    let dwdesiredaccess = GENERIC_READ;
    let dwsharemode = FILE_SHARE_READ;
    let dwcreationdisposition = OPEN_EXISTING;
    let drive: HANDLE = unsafe {
        // SAFETY - lpfilename & pcreateexparams remain valid while raw pointers are in use
        CreateFile2(
            lpfilename.as_pcwstr(),
            dwdesiredaccess,
            dwsharemode,
            dwcreationdisposition,
            null(),
        )
    };
    dbg!(&drive);
    let drive = validate(drive)?;
    let toc_command = CDROM_READ_TOC_EX::default();
    let mut toc = CDROM_TOC_FULL_TOC_DATA::default();
    let mut bytes_returned: u32 = 0;
    let read_toc = unsafe {
        // SAFETY: matching input & return buffer types
        DeviceIoControl(
            drive,
            IOCTL_CDROM_READ_TOC_EX,
            &toc_command as *const _ as *const _,
            size_of_val(&toc_command) as u32,
            &mut toc as *mut _ as *mut _,
            size_of_val(&toc) as u32,
            &mut bytes_returned as *mut _,
            null_mut(),
        )
    };  
    dbg!(read_toc);
    dbg!(bytes_returned);
    dbg!(size_of_val(&toc));
    dbg!(toc.Length);
    dbg!(toc.FirstCompleteSession);
    dbg!(toc.LastCompleteSession);
    for track in toc.Descriptors {
        dbg!(track.SessionNumber);
        dbg!(track.Msf);
        dbg!(track.MsfExtra);
        dbg!(track.Point);
        dbg!(track.Zero);
        dbg!(track._bitfield);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cda {
    pub version: u16,
    pub track_number: u16,
    pub windows_identifier: u32,
    pub range_offset_frames: u32,
    pub duration_frames: u32,
    pub range_position: CdTime,
    pub duration: CdTime,
}

impl TryFrom<Vec<u8>> for Cda {
    type Error = io::Error;

    fn try_from(data: Vec<u8>) -> Result<Self, Self::Error> {
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
        let duration_frames = u32::from_le_bytes([data[0x20], data[0x21], data[0x22], data[0x23]]);

        // Parse range position
        let range_position = CdTime {
            frame: data[0x24] as i8,
            sec: data[0x25] as i8,
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

        Ok(Cda {
            version,
            track_number,
            windows_identifier,
            range_offset_frames,
            duration_frames,
            range_position,
            duration,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CdTime {
    pub min: i8,
    pub sec: i8,
    pub frame: i8,
}

use windows_sys::{
    Win32::{
        Devices::Cdrom::{
            CDROM_READ_TOC_EX, CDROM_READ_TOC_EX_FORMAT_FULL_TOC, CDROM_TOC_FULL_TOC_DATA,
            IOCTL_CDROM_READ_TOC_EX,
        },
        Foundation::{GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CREATEFILE2_EXTENDED_PARAMETERS, CreateFile2, FILE_SHARE_READ, OPEN_EXISTING, ReadFile,
        },
        System::IO::{DeviceIoControl, OVERLAPPED},
    },
    core::PCWSTR,
};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
/// A somewhat sane way of dealing with `PWSTR/PCWSTR`: A pointer to a null terminated string
/// consisting of 'wide chars' (u16), encoded using UTF-16.
///
/// Construct via `WinString::from(&str)`
struct WinString {
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
    /// SAFETY: You must ensure that the returned `PCWSTR` is not used after self is dropped.
    /// It is recommended to call this directly in the call to a WinAPI unsafe function
    unsafe fn as_pcwstr(&self) -> PCWSTR {
        self.words.as_ptr()
    }
}

fn validate(handle: HANDLE) -> io::Result<HANDLE> {
    if handle == INVALID_HANDLE_VALUE {
        // If the function fails, the return value is INVALID_HANDLE_VALUE. To get extended error information, call GetLastError.
        return Err(io::Error::last_os_error());
    };
    Ok(handle)
}
