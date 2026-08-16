#![feature(never_type)]
#![feature(try_blocks)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use clap::Parser;
use exit_safely::Termination;
use redbook::{AudioCd, AudioCdExt, into_wav};
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

    // Determine track number - prompt if not provided
    let track_number = if let Some(n) = ripper.track_number {
        n
    } else {
        // No track number provided - prompt user
        let release = cd.disc()
            .selected_release()
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "No releases found"))?;
        
        let tracks = release
            .media
            .as_ref()
            .unwrap()
            .first()
            .unwrap()
            .tracks
            .as_ref()
            .unwrap();

        println!("\nAvailable tracks:");
        println!("0. All tracks");
        for track in tracks.iter() {
            let track_number: usize = track.number.as_ref().and_then(|n| n.parse().ok()).unwrap();
            let track_name = track.title.as_deref().unwrap_or("Unknown");
            println!("{}. {} - {}", track_number, track_number, track_name);
        }

        loop {
            let mut input = String::new();
            println!("\nEnter the track number to rip (0 for all):");

            io::stdin().read_line(&mut input).map_err(|error| {
                Exit::IO(format!("Problem reading input: {error}"))
            })?;

            let choice: usize = input.trim().parse().map_err(|_| {
                Exit::InvocationError(format!("Invalid track number: {}", input.trim()))
            })?;

            // Validate choice - 0 is valid (all), otherwise must be a valid track
            if choice == 0 || tracks.iter().any(|t| t.number.as_ref().and_then(|n| n.parse().ok()) == Some(choice)) {
                break choice;
            } else {
                println!("oops ... I can't find track number {choice}");
                continue;
            }
        }
    };

    // Handle rip all or single track
    let cover_art = cd.disc_mut().cover_art()?.clone().unwrap_or_default();
    
    if track_number == 0 {
        // Rip all tracks
        let ripped_tracks = cd.rip_all()?;
        
        for track in &ripped_tracks {
            let output_filename = ripper.output_filename(track.track_name.clone());
            let mut dump = File::create_new(&output_filename)?;
            let wav = into_wav(track.raw_data.clone());
            dump.write_all(&wav)?;
            println!("Track {} ripped to {}", track.track_number, output_filename);
        }
        
        // Save cover art once
        let mut cover = File::create_new("front.jpeg")?;
        cover.write_all(&cover_art)?;
        println!("Cover art saved to front.jpeg");
    } else {
        // Rip single track
        let ripped_track = cd.rip(track_number)?;
        let output_filename = ripper.output_filename(ripped_track.track_name.clone());

        let mut cover = File::create_new("front.jpeg")?;
        cover.write_all(&cover_art)?;

        let mut dump = File::create_new(&output_filename)?;
        let wav = into_wav(ripped_track.raw_data);
        dump.write_all(&wav)?;
        println!("Track {} ripped to {}", ripped_track.track_number, output_filename);
    }

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
