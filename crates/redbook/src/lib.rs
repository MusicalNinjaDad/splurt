//! CDDA CD digital audio as per RedBook (IEC 60908:1999)

use std::io;

use windows_sys::{
    Win32::{
        Foundation::{GENERIC_READ, HANDLE, INVALID_HANDLE_VALUE},
        Storage::FileSystem::{
            CREATEFILE2_EXTENDED_PARAMETERS, CreateFile2, FILE_SHARE_READ, OPEN_EXISTING,
        },
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
        todo!("invalid handle")
    };
    Ok(handle)
}

pub fn read_toc(drive: &str) -> io::Result<()> {
    let lpfilename = WinString::from(drive);
    let dwdesiredaccess = GENERIC_READ;
    let dwsharemode = FILE_SHARE_READ;
    let dwcreationdisposition = OPEN_EXISTING;
    let pcreateexparams = CREATEFILE2_EXTENDED_PARAMETERS::default();
    let drive_handle: HANDLE = unsafe {
        // SAFETY - lpfilename & pcreateexparams remain valid while raw pointers are in use
        CreateFile2(
            lpfilename.as_pcwstr(),
            dwdesiredaccess,
            dwsharemode,
            dwcreationdisposition,
            &pcreateexparams as *const _,
        )
    };
    let drive_handle = validate(drive_handle)?;
    Ok(())
}
