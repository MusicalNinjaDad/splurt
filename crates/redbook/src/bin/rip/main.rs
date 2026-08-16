#![feature(never_type)]
#![feature(try_blocks)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use clap::Parser;
use exit_safely::Termination;
use redbook::{AudioCd, AudioCdExt, into_wav, rip};
use std::{
    convert::Infallible,
    fs::File,
    io::{self, Write},
    path::PathBuf,
    process::Termination as _T,
    str::FromStr,
};
use try_v2::Try;

mod cli;
use cli::Rip;

use clap::Error as ClapError;

fn main() -> Exit<()> {
    let ripper = Rip::parse();

    let drive = PathBuf::from_str(&ripper.drive)?;
    let mut cd = AudioCd::new(drive)?;

    let track_number = ripper.track_number;
    let use_latest = ripper.non_interactive;

    if use_latest {
        cd.disc_mut().use_latest_release();
    }

    // closure is only called if release not set / obvious
    let _ = cd.disc_mut().get_or_select_release(|releases| {
        // Interactive selection - prompt user and loop on invalid input
        println!("Multiple releases found. Please select one:");
        releases.iter().enumerate().for_each(|(i, release)| {
            let date = release.date.as_deref().unwrap_or("unknown");
            let country = release.country.as_deref().unwrap_or("unknown");
            let barcode = release.barcode.as_deref().unwrap_or("none");
            println!(
                "{}. Date: {}, Country: {}, Barcode: {}",
                i + 1,
                date,
                country,
                barcode
            );
        });

        loop {
            #[expect(unused, reason = "loop on error")]
            try {
                let mut input = String::new();
                println!("\nEnter the number of the release to use:");

                let _ = io::stdin().read_line(&mut input).map_err(|error| {
                    println!("oops ... problem understanding you ... it's me, not you. {error}");
                })?;

                let choice = input.trim().parse::<usize>().map_err(|error| {
                    println!("oops ... try again {input} is not a number");
                })?;

                let index = choice - 1;
                releases.get(index).ok_or_else(|| {
                    println!("oops ... I can't find release number {choice}");
                });

                return Some(index);
            };
        }
    });

    let (track_name, pcm, cover_art) = rip(&mut cd, track_number)?;
    let output_filename = ripper.output_filename(track_name);

    let mut cover = File::create_new("front.jpeg")?;
    cover.write_all(&cover_art)?;

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

impl<T: _T> From<Infallible> for Exit<T> {
    fn from(_: Infallible) -> Self {
        unreachable!()
    }
}
