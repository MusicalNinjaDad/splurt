//! CDDA CD digital audio as per RedBook (IEC 60908:1999)

use std::{
    fs::OpenOptions,
    io::{self, Read},
};

pub fn read_toc(drive: &str) -> io::Result<()> {
    dbg!(&drive);
    let drive = OpenOptions::new().read(true).create(false).open(drive);
    dbg!(&drive);
// [crates/redbook/src/lib.rs:11:5] &drive = Err(
//     Os {
//         code: 5,
//         kind: PermissionDenied,
//         message: "Access is denied.",
//     },
// )
    const BYTES_TO_READ: usize = 32;
    let mut toc = [0u8; BYTES_TO_READ];
    let bytes_read = drive?.read(&mut toc)?;
    dbg!(bytes_read);
    dbg!(toc);
    Ok(())
}
