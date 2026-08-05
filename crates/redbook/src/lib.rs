//! CDDA CD digital audio as per RedBook (IEC 60908:1999)

use std::{
    fs::OpenOptions,
    io::{self, Read},
};

use cd_da_reader::CdReader;

pub fn read_toc(drive: &str) -> io::Result<()> {
    dbg!(&drive);
    match CdReader::open(drive) {
        Ok(_) => Ok(()),
        Err(error) => Err(io::Error::other(error)),
    }

    // [crates/redbook/src/lib.rs:11:5] &drive = "C:"
    // Device NOT opened succesfully

    // thread 'main' (38736) panicked at crates/redbook/src/lib.rs:12:40:
    // called `Result::unwrap()` on an `Err` value: Os { code: 5, kind: PermissionDenied, message: "Access is denied." }
}
