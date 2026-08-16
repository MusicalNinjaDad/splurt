#![feature(never_type)]
#![feature(try_blocks)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use clap::Parser;
use exit_safely::Termination;
use redbook::{AudioCd, AudioCdExt, into_wav, RippedTrack};
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

    // Determine what to rip - CLI flag or interactive prompt for track selection
    let (rip_all, track_number) = if ripper.all {
        (true, 0)
    } else if ripper.track_number.is_none() {
        // No track number on CLI triggers interactive track selection
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
        for track in tracks.iter() {
            let track_number: usize = track.number.as_ref().and_then(|n| n.parse().ok()).unwrap();
            let track_name = track.title.as_deref().unwrap_or("Unknown");
            println!("{}. {}", track_number, track_name);
        }
        println!("a. All tracks");

        loop {
            #[expect(unused, reason = "loop on error")]
            try {
                let mut input = String::new();
                println!("\nEnter the track number to rip (a for all):");

                let _ = io::stdin().read_line(&mut input).map_err(|error| {
                    println!("oops ... problem understanding you ... it's me, not you. {error}");
                })?;

                let input_trimmed = input.trim().to_lowercase();
                if input_trimmed == "a" {
                    break (true, 0);
                }

                let choice: usize = input_trimmed.parse().map_err(|error| {
                    println!("oops ... try again {input_trimmed} is not a number");
                })?;

                let valid = tracks.iter().any(|t| t.number.as_ref().and_then(|n| n.parse().ok()) == Some(choice));
                valid.ok_or_else(|| {
                    println!("oops ... I can't find track number {choice}");
                })?;

                break (false, choice);
            };
        }
    } else {
        (false, ripper.track_number.unwrap())
    };

    // Write cover art once before ripping
    let mut cover = File::create_new("front.jpeg")?;
    if let Some(data) = cd.disc_mut().cover_art()? {
        cover.write_all(data)?;
    }

    // Rip tracks
    let ripped_tracks: Vec<RippedTrack> = if rip_all {
        cd.rip_all()?
    } else {
        vec![cd.rip(track_number)?]
    };

    // Write WAV files
    for track in &ripped_tracks {
        let track_name = cd
            .disc()
            .selected_release()
            .unwrap_or_else(|| 
                panic!("No release found - this should have been caught earlier"))
            .media
            .as_ref()
            .unwrap()
            .first()
            .unwrap()
            .tracks
            .as_ref()
            .unwrap()
            .iter()
            .find(|t| t.number.as_ref().and_then(|n| n.parse().ok()) == Some(track.track_number))
            .and_then(|t| t.title.as_ref())
            .map(|t| t.as_str())
            .unwrap_or("");

        let output_filename = if track_name.is_empty() {
            format!("{:02}.wav", track.track_number)
        } else {
            format!("{:02} {}.wav", track.track_number, track_name)
        };

        let mut dump = File::create_new(&output_filename)?;
        let wav = into_wav(track.raw_data.clone());
        dump.write_all(&wav)?;
        println!("Track {} ripped to {}", track.track_number, output_filename);
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
