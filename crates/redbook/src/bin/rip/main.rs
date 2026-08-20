#![feature(never_type)]
#![feature(try_blocks)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use clap::Parser;
use exit_safely::Termination;
use flacenc::{component::BitRepr, error::Verify};
use metaflac::Tag;
use redbook::{AudioCd, AudioCdExt};
use std::{
    convert::Infallible,
    fs::File,
    io::{self, Write},
    path::PathBuf,
    process::Termination as _T,
    str::FromStr,
};
use try_v2::Try;

#[derive(Debug, Clone, Copy)]
enum SelectedTrack {
    All,
    One(usize),
}

mod cli;
use cli::Rip;

use clap::Error as ClapError;

fn main() -> Exit<()> {
    let ripper = Rip::parse();

    let drive = PathBuf::from_str(&ripper.drive)?;
    let mut cd = AudioCd::new(drive)?;

    if cd.disc().release().is_none() {
        cd.disc_mut().select_release(|releases| {
            if ripper.non_interactive {
                releases
                    .iter()
                    .max_by_key(|release| &release.date)
                    .and_then(|release| releases.iter().position(|r| r.id == release.id))
            } else {
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
                            println!(
                                "oops ... problem understanding you ... it's me, not you. {error}"
                            );
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
            }
        });
    };

    let selected_track = match (ripper.all, ripper.track_number) {
        (true, Some(_)) => {
            return Exit::InvocationError(
                "Cannot specify both --all and a track number".to_string(),
            );
        }
        (true, None) => SelectedTrack::All,
        (false, Some(n)) => SelectedTrack::One(n),
        (false, None) => {
            println!("\nAvailable tracks:");

            for track in cd.disc().tracks() {
                let track_name = track.title().unwrap_or_else(|| "Unknown".to_string());
                println!("{n}. {track_name}", n = track.toc_entry.track);
            }
            println!("a. All tracks");

            loop {
                #[expect(unused, reason = "loop on error")]
                try {
                    let mut input = String::new();
                    println!("\nEnter the track number to rip (a for all):");

                    let _ = io::stdin().read_line(&mut input).map_err(|error| {
                        println!(
                            "oops ... problem understanding you ... it's me, not you. {error}"
                        );
                    })?;

                    let input_trimmed = input.trim().to_lowercase();
                    if input_trimmed == "a" {
                        break SelectedTrack::All;
                    }

                    let choice: usize = input_trimmed.parse().map_err(|error| {
                        println!("oops ... try again {input_trimmed} is not a number");
                    })?;

                    let valid = cd
                        .disc()
                        .tracks()
                        .any(|t| t.toc_entry.track as usize == choice);
                    valid.ok_or_else(|| {
                        println!("oops ... I can't find track number {choice}");
                    })?;

                    break SelectedTrack::One(choice);
                };
            }
        }
    };

    if let Some(data) = cd.disc().cover_art() {
        let mut cover = File::create_new("front.jpeg")?;
        cover.write_all(data)?
    }

    let track_number = match selected_track {
        SelectedTrack::All => todo!("implement rip all"),
        SelectedTrack::One(n) => n,
    };

    let ripped = cd.rip(track_number)?;

    // define just in time, to allow for mutable borrows earlier
    let track = cd.disc().track(track_number).unwrap();
    let output_filename = PathBuf::from(
        [
            format!("{:02}", track_number),
            track.title().unwrap_or_default(),
        ]
        .join(" "),
    );

    let (channels, bits_per_sample, sample_rate) = (2, 16, 44100);
    let config = flacenc::config::Encoder::default()
        .into_verified()
        .expect("Config data error.");
    let samples: Vec<_> = ripped
        .raw_data
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as i32)
        .collect();
    let source =
        flacenc::source::MemSource::from_samples(&samples, channels, bits_per_sample, sample_rate);
    let flac_stream = flacenc::encode_with_fixed_block_size(&config, source, config.block_size)
        .expect("Encode failed.");
    let mut sink = flacenc::bitsink::ByteSink::new();
    flac_stream.write(&mut sink).unwrap();
    let flac_filename = output_filename.with_extension("flac");
    {
        let mut dump = File::create_new(&flac_filename)?;
        dump.write_all(sink.as_slice())?;
        println!(
            "Track {} ripped to {}",
            track.track_number(),
            output_filename.display()
        );
    }

    if let Some(tags) = cd.disc().tag_for(track_number) {
        let mut tag = Tag::read_from_path(&flac_filename).unwrap();
        let vorbis = tag.vorbis_comments_mut();
        vorbis.comments.extend(tags.comments);
        dbg!(&tag);
        tag.write_to_path(&flac_filename).unwrap();
    }
    let written_tag = Tag::read_from_path(&flac_filename).unwrap();
    dbg!(written_tag);

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
