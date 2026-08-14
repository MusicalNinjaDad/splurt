#![feature(exact_div)]

//! CDDA CD digital audio as per RedBook (IEC 60908:1999)

use std::{
    fs::{self, read_dir},
    io,
    path::PathBuf,
    ptr::{null, null_mut},
};

use std::convert::TryFrom;

const FRAME_SIZE: usize = 2352;
const MAX_CHUNK_FRAMES: usize = 64 * 1024 / FRAME_SIZE;
const MAX_CHUNK_BYTES: usize = MAX_CHUNK_FRAMES * FRAME_SIZE;

pub fn read_toc(drive: &str) -> io::Result<()> {
    let drive: PathBuf = drive.into();
    let cdas: Vec<_> = read_dir(&drive)?
        .map(|track| Cda::try_from(fs::read(track.unwrap().path()).unwrap()).unwrap())
        .collect();
    dbg!(&cdas);
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

    let track1 = cdas[0];
    let track_size = usize::try_from(track1.duration_frames)
        .unwrap()
        .strict_mul(FRAME_SIZE);
    debug_assert!(track_size > 0);
    let mut frame = vec![0_u8; track_size];
    dbg!(frame.len());

    let (bufs, last_buf) = frame.as_chunks_mut::<MAX_CHUNK_BYTES>();
    // last_buf SAFETY - must have at least capacity for SectorCount

    let mut bytes_read_so_far = 0_i64;
    for (i, buf) in bufs.iter_mut().enumerate() {
        // SAFETY - must match size of buffer
        let frames_to_read: u32 = MAX_CHUNK_FRAMES.try_into().unwrap();
        let bytes_to_read: u32 = MAX_CHUNK_BYTES.try_into().unwrap();

        debug_assert_eq!(
            bytes_read_so_far,
            i.strict_mul(MAX_CHUNK_BYTES).try_into().unwrap(),
        );
        let offset = track1.offset().strict_add(bytes_read_so_far);
        let bytes_read = read_chunk(drive, offset, bytes_to_read, frames_to_read, buf)?;
        bytes_read_so_far += &bytes_read.into();
    }

    let frames_read_so_far = bufs.len().strict_mul(MAX_CHUNK_FRAMES);
    debug_assert_eq!(
        i64::try_from(frames_read_so_far)
            .unwrap()
            .strict_mul(FRAME_SIZE.try_into().unwrap()),
        bytes_read_so_far
    );
    let frames_to_read = track1
        .duration_frames
        .strict_rem(frames_read_so_far.try_into().unwrap());
    let bytes_to_read = last_buf.len().try_into().unwrap();
    debug_assert_eq!(
        i64::from(bytes_to_read),
        i64::from(track1.duration_frames)
            .strict_mul(FRAME_SIZE.try_into().unwrap())
            .strict_sub(bytes_read_so_far)
    );

    let offset = track1.offset().strict_add(bytes_read_so_far);
    let bytes_read = read_chunk(drive, offset, bytes_to_read, frames_to_read, last_buf)?;
    bytes_read_so_far += &bytes_read.into();

    dbg!(bytes_read_so_far);
    Ok(())
}

fn read_chunk(
    drive: HANDLE,
    offset: i64,
    bytes_to_read: u32,
    frames_to_read: u32,
    buf: &mut [u8],
) -> io::Result<u32> {
    let read_command = RAW_READ_INFO {
        DiskOffset: offset,
        SectorCount: frames_to_read,
        TrackMode: 2, // CDDA(?) https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddcdrm/ne-ntddcdrm-_track_mode_type
    };

    let mut bytes_read: u32 = 0;
    dbg!(offset);

    // SAFETY - inline based on https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddcdrm/ni-ntddcdrm-ioctl_cdrom_raw_read
    let read_chunk = unsafe {
        // SAFETY: Buffer is expected size
        debug_assert_eq!(bytes_to_read, buf.len().try_into().unwrap());
        // SAFETY: Buffer is exact size for Sector count
        debug_assert_eq!(
            read_command.SectorCount,
            bytes_to_read
                .div_exact(FRAME_SIZE.try_into().unwrap())
                .expect("no remainder")
        );

        DeviceIoControl(
            drive,
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
    debug_assert_eq!(bytes_read, bytes_to_read);
    Ok(bytes_read)
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

impl Cda {
    /// Contains an offset into the CD-ROM disc where the track starts. You can calculate this offset by multiplying the starting sector number for the request times 2048.
    /// See https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddcdrm/ns-ntddcdrm-__raw_read_info
    fn offset(&self) -> i64 {
        (self.range_offset_frames as i64 + 150)
            * i64::try_from(FRAME_SIZE).expect("FRAME_SIZE is positive")
    }
}

use windows_sys::{
    Win32::{
        Devices::Cdrom::{
            CDROM_READ_TOC_EX, CDROM_READ_TOC_EX_FORMAT_FULL_TOC, CDROM_TOC_FULL_TOC_DATA,
            IOCTL_CDROM_RAW_READ, IOCTL_CDROM_READ_TOC_EX, RAW_READ_INFO, TRACK_MODE_TYPE,
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
