#![feature(never_type)]
#![feature(try_blocks)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use std::{
    convert::Infallible,
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
    process::Termination as _T,
    str::FromStr,
    sync::mpsc,
    thread,
};

use clap::Parser;
use exit_safely::Termination;
use metaflac::{
    Block, Tag,
    block::{Picture, PictureType},
};
use redbook::{AudioCd, AudioCdExt, AudioCdExtMut, RippedTrack, tagging::PictureExt};
use tracing::{debug, info, info_span};
use try_v2::Try;

#[derive(Debug, Clone, Copy)]
enum SelectedTrack {
    All,
    One(usize),
}

mod _tracing;
mod cli;
pub(crate) use cli::Rip;

use clap::Error as ClapError;

fn main() -> Exit<()> {
    let ripper = Rip::try_parse()?;

    ripper.init_tracing()?;

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

    let disc_title = cd.disc().title().unwrap_or_else(|| "Unknown".to_string());

    let _album_span = info_span!("rip_album", album = %disc_title).entered();

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

    let disc_title = cd.disc().title().unwrap_or_else(|| "Unknown".to_string());
    let artist = cd
        .disc()
        .main_artist()
        .unwrap_or_else(|| "Unknown".to_string());
    let output_dir = PathBuf::from(artist).join(disc_title);
    fs::create_dir_all(&output_dir)?;

    let _ = cd.disc_mut().update_cover_art();
    if let Some(Err(error_saving_coverart)) = cd.disc().save_cover_art(&output_dir) {
        dbg!(error_saving_coverart);
    };

    let cd = cd.lock();
    let disc = cd.disc().clone();

    let (ripped_tracks_tx, ripped_tracks_rx) = mpsc::channel::<RippedTrack>();

    let ripper = thread::spawn(move || {
        for track_number in track_numbers.clone() {
            try {
                let track = cd.disc().track(track_number).unwrap();
                let track_name = track.title().unwrap_or_default();
                info!(
                    target: "rip",
                    track = track_number,
                    name = %track_name,
                    "rip_track_start"
                );
                let start = std::time::Instant::now();
                let ripped = cd.rip(track_number).ok()?;
                let duration = start.elapsed();
                info!(
                    target: "rip",
                    track = track_number,
                    name = %track_name,
                    duration_secs = ?duration.as_secs_f64(),
                    "rip_track_done"
                );
                ripped_tracks_tx.send(ripped).ok()?;
            };
        }
    });

    let encoder = thread::spawn(move || {
        let enc = try {
            while let Ok(ripped) = ripped_tracks_rx.recv() {
                let track_number = ripped.track_number;
                let track = disc.track(track_number).unwrap();
                let track_name = track.title().unwrap_or_default();

                debug!(
                    target: "encode",
                    track = track_number,
                    name = %track_name,
                    "encode_start"
                );
                let start = std::time::Instant::now();

                let flac_path = output_dir.join(track.filename()).with_extension("flac");
                let flac = ripped.to_flac();
                let mut flac_file = File::create_new(&flac_path)?;
                flac_file.write_all(flac.as_slice())?;
                println!(
                    "Track {} ripped to {}",
                    track.track_number(),
                    flac_path.display()
                );
                let bytes_written = flac.as_slice().len();

                let mut tag = Tag::read_from_path(&flac_path).unwrap();
                if let Some(tags) = disc.tag_for(track_number) {
                    let vorbis = tag.vorbis_comments_mut();
                    vorbis.comments.extend(tags.comments);
                }

                if let Some(cover) =
                    disc.cover_art()
                        .cloned()
                        .or(fs::read(output_dir.join("front.jpeg")).ok().map(|data| {
                            Picture::from_jpeg(PictureType::CoverFront, "Front Cover", data)
                        }))
                {
                    tag.push_block(Block::Picture(cover));
                }

                tag.write_to_path(&flac_path).unwrap();

                let duration = start.elapsed();
                debug!(
                    target: "encode",
                    track = track_number,
                    bytes = bytes_written,
                    duration_secs = ?duration.as_secs_f64(),
                    "encode_done"
                );
            }
        };
        drop(ripped_tracks_rx);
        enc
    });

    ripper
        .join()
        .map_err(|panicked| Exit::Error(format!("ripping panicked: {panicked:?}")))?;
    encoder
        .join()
        .map_err(|panicked| Exit::Error(format!("encoding panicked: {panicked:?}")))??;

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
    Logging(String) = 4,
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
