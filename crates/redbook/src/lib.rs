#![feature(exact_div)]

//! CDDA CD digital audio as per RedBook (IEC 60908:1999)

use std::{
    convert::TryFrom,
    fs::{self, read_dir},
    io,
    path::PathBuf,
    ptr::{null, null_mut},
};

use windows_sys::{
    Win32::{
        Devices::Cdrom::{IOCTL_CDROM_RAW_READ, RAW_READ_INFO, TRACK_MODE_TYPE},
        Foundation::{GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{CreateFile2, FILE_SHARE_READ, OPEN_EXISTING},
        System::IO::DeviceIoControl,
    },
    core::PCWSTR,
};

/// One cdda audio frame in bytes
const FRAME_SIZE: usize = 2352;

// If chunks are too large DeviceIoControl(.., IOCTL_CDROM_RAW_READ,..) fails.
// Calc frames first, then reverse calc bytes as we need an exact number of frames.
// TODO: research max chunk size. Guessing 64k for now based on something I saw in cd_da_reader but with no references given
const MAX_CHUNK_FRAMES: usize = 64 * 1024 / FRAME_SIZE;
const MAX_CHUNK_BYTES: usize = MAX_CHUNK_FRAMES * FRAME_SIZE;

//(?) https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/ntddcdrm/ne-ntddcdrm-_track_mode_type
const TRACK_MODE_CDDA: TRACK_MODE_TYPE = 2;

pub fn grab_track(drive: &str) -> io::Result<Vec<u8>> {
    let drive: PathBuf = drive.into();

    // Windows already helpfully decodes the TOC for us. Parsing .cda files avoids calling
    // ugly & unsafe ffi functions and wrangling the returned, nested structs.
    let cdas: Vec<_> = read_dir(&drive)?
        .map(|track| Cda::try_from(fs::read(track.unwrap().path()).unwrap()).unwrap())
        .collect();
    dbg!(&cdas);

    // Need to use ffi to raw read data. No API found to "get track audio data".
    // Using .strict_... for all conversions; liberal application of debug_asserts.
    let windrive = format!(r"\\.\{}", drive.display());
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
    dbg!(&drive);
    let drive = validate(drive)?;

    // For now just grab whichever track is shortest on the CD I have in right now (manually identified)
    let track = cdas[8];
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
        // SAFETY note - must match each other and size of buffer
        let frames_to_read: u32 = MAX_CHUNK_FRAMES.try_into().unwrap();
        let bytes_to_read: u32 = MAX_CHUNK_BYTES.try_into().unwrap();

        debug_assert_eq!(
            bytes_read_so_far,
            i.strict_mul(MAX_CHUNK_BYTES).try_into().unwrap(),
            "now reading chunk {i} but have only read {bytes_read_so_far} bytes so far"
        );
        let offset = track.offset().strict_add(bytes_read_so_far);
        let bytes_read = read_chunk(drive, offset, bytes_to_read, frames_to_read, buf)?;
        bytes_read_so_far += i64::from(bytes_read);
    }

    let frames_read_so_far = bufs.len().strict_mul(MAX_CHUNK_FRAMES);
    debug_assert_eq!(
        i64::try_from(frames_read_so_far)
            .unwrap()
            .strict_mul(FRAME_SIZE.try_into().unwrap()),
        bytes_read_so_far,
        "about to read last frame. We have read {frames_read_so_far} frames, but only {bytes_read_so_far} bytes so far"
    );
    let frames_to_read = track
        .duration_frames
        .strict_rem(MAX_CHUNK_FRAMES.try_into().unwrap());
    let bytes_to_read = last_buf.len().try_into().unwrap();
    debug_assert_eq!(
        i64::from(bytes_to_read),
        i64::from(track.duration_frames)
            .strict_mul(FRAME_SIZE.try_into().unwrap())
            .strict_sub(bytes_read_so_far),
        "about to read last frame. {bytes_to_read} bytes remaining, this seems does not match number of frames read so far"
    );

    let offset = track.offset().strict_add(bytes_read_so_far);
    let bytes_read = read_chunk(drive, offset, bytes_to_read, frames_to_read, last_buf)?;
    bytes_read_so_far += i64::from(bytes_read);

    dbg!(bytes_read_so_far);
    Ok(data)
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
        TrackMode: TRACK_MODE_CDDA,
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
    debug_assert_eq!(
        bytes_read, bytes_to_read,
        "intended to read {bytes_to_read} bytes from offset {offset} but only got {bytes_read}"
    );
    Ok(bytes_read)
}

pub fn into_wav(pcm: Vec<u8>) -> Vec<u8> {
    // based on cd_da_reader
    let pcm_data_size = pcm.len();
    let mut wav = Vec::with_capacity(44 + pcm_data_size);

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
pub struct Cda {
    pub track_number: u16,
    pub windows_identifier: u32,
    /// First frame *relative to end of lead-in* (150 frames less than starting_time)
    /// Assuming Windows does it this way, which appears backwards, for historical reasons!
    ///
    /// For example dbg!() from a real Cda gives:
    /// ```text
    /// Cda {
    ///     track_number: 1,
    ///     windows_identifier: 17596852,
    ///     starting_frame: 33,
    ///     duration_frames: 24242,
    ///     starting_time: CdTime {
    ///         min: 0,
    ///         sec: 2,
    ///         frame: 33,
    ///     },
    ///     duration: CdTime {
    ///         min: 5,
    ///         sec: 23,
    ///         frame: 17,
    ///     },
    /// }
    /// ```
    pub starting_frame: u32,
    pub duration_frames: u32,
    /// *Absolute* starting time, track 1 will be >= 2sec due to lead-in
    /// Assuming Windows does it this way, which appears backwards, for historical reasons!
    ///
    /// For example dbg!() from a real Cda gives:
    /// ```text
    /// Cda {
    ///     track_number: 1,
    ///     windows_identifier: 17596852,
    ///     starting_frame: 33,
    ///     duration_frames: 24242,
    ///     starting_time: CdTime {
    ///         min: 0,
    ///         sec: 2,
    ///         frame: 33,
    ///     },
    ///     duration: CdTime {
    ///         min: 5,
    ///         sec: 23,
    ///         frame: 17,
    ///     },
    /// }
    /// ```
    pub starting_time: CdTime,
    pub duration: CdTime,
}

/// Parsing based on https://en.wikipedia.org/wiki/.cda_file
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
            track_number,
            windows_identifier,
            starting_frame: range_offset_frames,
            duration_frames,
            starting_time: range_position,
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
    /// Contains an offset into the CD-ROM disc where the track starts in [`FRAME_SIZE`]-byte sectors.
    fn offset(&self) -> i64 {
        (self.starting_frame as i64 + 150)
            * i64::try_from(FRAME_SIZE).expect("FRAME_SIZE is positive")
    }
}

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
