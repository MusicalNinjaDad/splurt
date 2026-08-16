#![feature(never_type)]
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

    cd.disc_mut().get_or_select_release(|releases| {
        if use_latest {
            Ok(releases
                .iter()
                .enumerate()
                .max_by(|(_, a), (_, b)| {
                    a.date
                        .as_deref()
                        .partial_cmp(&b.date.as_deref())
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i)
                .unwrap_or(0))
        } else {
            // Interactive selection - prompt user
            println!("Multiple releases found. Please select one:");
            releases.iter().enumerate().for_each(|(i, release)| {
                let date = release.date.as_deref().unwrap_or("unknown");
                let country = release.country.as_deref().unwrap_or("unknown");
                let barcode = release.barcode.as_deref().unwrap_or("none");
                println!("{}. Date: {}, Country: {}, Barcode: {}", i + 1, date, country, barcode);
            });
            
            let mut input = String::new();
            println!("Enter the number of the release to use (1-{}):", releases.len());
            io::stdin().read_line(&mut input)?;
            
            let choice = input.trim().parse::<usize>()
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid selection"))?;
            
            if choice > 0 && choice <= releases.len() {
                Ok(choice - 1)
            } else {
                Err(io::Error::new(io::ErrorKind::InvalidInput, "Selection out of range"))
            }
        }
    })?;

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
