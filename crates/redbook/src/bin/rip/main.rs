#![feature(never_type)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use clap::Parser;
use exit_safely::Termination;
use redbook::{AudioCd, AudioCdExt, into_wav, rip, select_release};
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

    // Check if we need to select a release
    if let Some(mb_data) = cd.disc().musicbrainz.as_ref() {
        if mb_data.releases.len() > 1 {
            let releases = &mb_data.releases;
            
            // Check if --latest flag is set
            if ripper.latest {
                if let Some(latest_index) = select_release(releases, true) {
                    cd.disc_mut().release_id = Some(releases[latest_index].id.clone());
                }
            } else {
                // Interactive selection - prompt user
                println!("Multiple releases found. Please select one:");
                for (i, release) in releases.iter().enumerate() {
                    let date = release.date.as_deref().unwrap_or("unknown");
                    let country = release.country.as_deref().unwrap_or("unknown");
                    let barcode = release.barcode.as_deref().unwrap_or("none");
                    println!("{}. Date: {}, Country: {}, Barcode: {}", i + 1, date, country, barcode);
                }
                
                let mut input = String::new();
                println!("Enter the number of the release to use (1-{}):", releases.len());
                io::stdin().read_line(&mut input)?;
                
                if let Ok(choice) = input.trim().parse::<usize>() {
                    if choice > 0 && choice <= releases.len() {
                        cd.disc_mut().release_id = Some(releases[choice - 1].id.clone());
                    } else {
                        return Exit::Error(format!("Invalid selection: {}", choice));
                    }
                } else {
                    return Exit::Error("Invalid input. Please enter a number.".to_string());
                }
            }
        } else if mb_data.releases.len() == 1 {
            // Only one release, use it
            cd.disc_mut().release_id = Some(mb_data.releases[0].id.clone());
        }
    }

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
