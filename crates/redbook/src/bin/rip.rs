#![feature(never_type)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use clap::Parser;
use exit_safely::Termination;
use redbook::{cli::Rip, grab_track, into_wav};
use std::{
    fs::File,
    io::{self, Write},
    process::Termination as _T,
};
use try_v2::Try;

use clap::Error as ClapError;

fn main() -> Exit<()> {
    let rip = Rip::parse();

    let drive = &rip.drive;
    let track_number = rip.track_number;
    let output_filename = rip.output_filename();

    let pcm = grab_track(drive, track_number)?;
    let mut dump = File::create_new(&output_filename)?;
    let wav = into_wav(pcm);
    dump.write_all(&wav)?;
    println!("Track {} ripped to {}", track_number, output_filename);

    Exit::Ok(())
}

#[derive(Debug, Termination, Try, PartialEq, PartialOrd, Eq, Ord)]
#[FromResidual(Result<_, Self::Residual>)]
#[repr(u8)]
#[must_use]
pub enum Exit<T: _T> {
    Ok(T) = 0,
    Error(String) = 1,
    InvocationError(String) = 2,
    IO(String) = 3,
}

impl<T: _T> From<ClapError> for Exit<T> {
    fn from(e: ClapError) -> Self {
        Self::InvocationError(e.to_string())
    }
}

impl<T: _T> From<io::Error> for Exit<T> {
    fn from(e: io::Error) -> Self {
        Self::IO(e.to_string())
    }
}
