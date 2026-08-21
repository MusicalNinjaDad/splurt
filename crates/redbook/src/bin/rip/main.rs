#![feature(never_type)]
#![feature(try_blocks)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use clap::Parser;
use exit_safely::Termination;
use metaflac::{
    Block, Tag,
    block::{Picture, PictureType},
};
use redbook::{AudioCd, AudioCdExt};
use std::{
    convert::Infallible,
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
    process::Termination as _T,
    str::FromStr,
};
use try_v2::Try;
use zune_jpeg::{JpegDecoder, zune_core::bytestream::ZCursor};

#[derive(Debug, Clone, Copy)]
enum SelectedTrack {
    All,
    One(usize),
}

mod cli;
use cli::Rip;

use clap::Error as ClapError;

fn main() -> Exit<()> {
    let ripper = Rip::try_parse()?;

    let drive = PathBuf::from_str(&ripper.drive)?;
    let mut cd = AudioCd::new(drive)?;

    let _ = cd.disc_mut().update_musicbrainz();
    if cd.disc().release().is_none() {
        cd.disc_mut().select_release(|releases| {
            if ripper.non_interactive {
                releases
                    .iter()
                    .max_by_key(|release| {
                        release
                            .date
                            .as_ref()
                            .map(|date| date.into_naive_date(9999, 12, 12).ok())
                    })
                    .and_then(|release| releases.iter().position(|r| r.id == release.id))
            } else {
                println!("Multiple releases found. Please select one:");
                releases.iter().enumerate().for_each(|(i, release)| {
                    let date = release
                        .date
                        .as_ref()
                        .map(ToString::to_string)
                        .unwrap_or("unknown".to_string());
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

    let track_numbers = match selected_track {
        SelectedTrack::All => 1..=cd.disc().tracks().len(),
        SelectedTrack::One(n) => n..=n,
    };

    let output_dir = PathBuf::from(cd.disc().title().unwrap_or_else(|| "Unknown".to_string()));

    let _ = cd.disc_mut().update_cover_art();
    if let Some(data) = cd.disc().cover_art() {
        let filename = output_dir.join("front.jpeg");
        let mut cover = File::create_new(filename)?;
        cover.write_all(data)?
    }

    for track_number in track_numbers {
        let ripped = cd.rip(track_number)?;

        // define just in time, to allow for mutable borrows earlier
        let track = cd.disc().track(track_number).unwrap();
        
        let output_path = output_dir.join(track.filename());

        let flac = ripped.to_flac();
        let flac_filename = output_path.with_extension("flac");
        let mut dump = File::create_new(&flac_filename)?;
        dump.write_all(flac.as_slice())?;
        println!(
            "Track {} ripped to {}",
            track.track_number(),
            output_path.display()
        );

        if let Some(tags) = cd.disc().tag_for(track_number) {
            let mut tag = Tag::read_from_path(&flac_filename).unwrap();
            let vorbis = tag.vorbis_comments_mut();
            vorbis.comments.extend(tags.comments);
            if let Some(image) = cd
                .disc()
                .cover_art()
                .map(|b| b.to_vec())
                .or(fs::read("front.jpeg").ok())
            {
                let mut jpg_info = JpegDecoder::new(ZCursor::from(&image));
                let _ = jpg_info.decode_headers();
                let width = jpg_info.info().unwrap().width as u32;
                let height = jpg_info.info().unwrap().height as u32;
                let cover = Picture {
                    picture_type: PictureType::CoverFront,
                    mime_type: "image/jpeg".to_string(),
                    description: "Front Cover".to_string(),
                    width,
                    height,
                    depth: 24,
                    num_colors: 0,
                    data: image,
                };
                tag.push_block(Block::Picture(cover));
            }
            dbg!(&tag);
            tag.write_to_path(&flac_filename).unwrap();
        }
        let written_tag = Tag::read_from_path(&flac_filename).unwrap();
        dbg!(written_tag);
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
