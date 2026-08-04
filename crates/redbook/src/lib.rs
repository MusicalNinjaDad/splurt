//! CDDA CD digital audio as per RedBook (IEC 60908:1999)

use std::{
    fs::File,
    io::{self, Read},
};

pub fn read_toc(drive: &str) -> io::Result<()> {
    let mut drive = File::open(drive)?;
    const BYTES_TO_READ: usize = 2048;
    let mut toc = [0u8; BYTES_TO_READ];
    let bytes_read = drive.read(&mut toc)?;
    dbg!(bytes_read);
    dbg!(toc);
    Ok(())
}
