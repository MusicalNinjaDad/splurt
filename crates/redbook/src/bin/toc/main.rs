#![allow(rust_analyzer::inactive_code)]

#[cfg(target_family = "windows")]
use redbook::hex::hex_dump;
#[cfg(target_family = "windows")]
use redbook::win::CdDrive;

#[cfg(target_family = "windows")]
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let drive = CdDrive::open(&args[1]).unwrap();
    let toc = drive.toc_as_raw_bytes();
    println!("{}", hex_dump(toc));
}

#[cfg(not(target_family = "windows"))]
fn main() {}
