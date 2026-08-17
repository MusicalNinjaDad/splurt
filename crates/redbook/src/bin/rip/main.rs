#![feature(never_type)]
#![feature(try_blocks)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use clap::Parser;
use exit_safely::Termination;
use flacenc::{component::BitRepr, error::Verify};
use metaflac::Tag;
use redbook::{
    AudioCd, AudioCdExt, RippedTrack, into_wav,
    musicbrainz::ArtistCreditsExt,
};
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

            let release = cd.disc().release();
            for track in cd.tracks() {
                let track_name = release
                    .and_then(|r| r.media.as_ref())
                    .and_then(|media| media.first())
                    .and_then(|media| media.tracks.as_ref())
                    .and_then(|tracks| {
                        tracks
                            .iter()
                            .find(|trk| {
                                trk.number.as_ref().and_then(|n| n.parse().ok())
                                    == Some(track.track_number)
                            })
                            .and_then(|trk| trk.title.as_ref())
                    })
                    .map(|title| title.as_str())
                    .unwrap_or("Unknown");
                println!("{n}. {track_name}", n = track.track_number);
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
                        .tracks()
                        .into_iter()
                        .any(|t| t.track_number as usize == choice);
                    valid.ok_or_else(|| {
                        println!("oops ... I can't find track number {choice}");
                    })?;

                    break SelectedTrack::One(choice);
                };
            }
        }
    };

    match cd.disc_mut().cover_art() {
        Ok(Some(data)) => {
            let mut cover = File::create_new("front.jpeg")?;
            cover.write_all(data)?
        }
        Ok(None) => {
            dbg!("No cover art found");
        }
        Err(err) => {
            dbg!(err);
        }
    };

    let ripped_tracks: Vec<RippedTrack> = match selected_track {
        SelectedTrack::All => cd.rip_all()?,
        SelectedTrack::One(n) => vec![cd.rip(n)?],
    };

    // define just in time, to allow for mutable borrows earlier
    let release = cd.disc().release();
    for track in &ripped_tracks {
        let mbtrk = release.and_then(|release| release.track(track.track_number));

        let output_filename = PathBuf::from(
            [
                format!("{:02}", track.track_number),
                track.track_name.clone(),
            ]
            .join(" "),
        );

        let mut dump = File::create_new(output_filename.with_extension("wav"))?;
        let wav = into_wav(track.raw_data.clone());
        dump.write_all(&wav)?;
        println!(
            "Track {} ripped to {}",
            track.track_number,
            output_filename.display()
        );

        let (channels, bits_per_sample, sample_rate) = (2, 16, 44100);
        let config = flacenc::config::Encoder::default()
            .into_verified()
            .expect("Config data error.");
        let samples: Vec<i32> = track
            .raw_data
            .chunks_exact(2)
            .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as i32)
            .collect();
        let source = flacenc::source::MemSource::from_samples(
            &samples,
            channels,
            bits_per_sample,
            sample_rate,
        );
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
                track.track_number,
                output_filename.display()
            );
        }
        let mut tag = Tag::read_from_path(&flac_filename).unwrap();
        let vorbis = tag.vorbis_comments_mut();
        vorbis.set_title(vec![track.track_name.clone()]);
        vorbis.set_track(track.track_number as u32);
        if let Some(release) = cd.disc().release() {
            vorbis.set(
                "SCRIPT",
                vec![
                    release
                        .text_representation
                        .as_ref()
                        .and_then(|text_rep| text_rep.script.clone())
                        .unwrap_or_default(),
                ],
            );

            vorbis.set(
                "MUSICBRAINZ_TRACKID",
                vec![mbtrk.and_then(|trk| trk.id.clone()).unwrap_or_default()],
            );

            vorbis.set_album(vec![release.title.clone()]);
            vorbis.set("MUSICBRAINZ_ALBUMID", vec![release.id.clone()]);

            vorbis.set_album_artist(release.artist_credit.names().collect());

            let track_artists = mbtrk
                .and_then(|trk| trk.artist_credit.as_ref())
                .or(release.artist_credit.as_ref());
            vorbis.set_artist(track_artists.names().collect());

            vorbis.set(
                "MUSICBRAINZ_ALBUMARTISTID",
                track_artists.artist_ids().collect(),
            );

            let release_date = release.date.clone().unwrap_or_default();
            let release_year = release_date.get(0..4).unwrap_or_default().to_string();
            vorbis.set("RELEASEDATE", vec![release_date]);
            vorbis.set("RELEASEYEAR", vec![release_year]);

            let original_date = mbtrk
                .and_then(|trk| trk.recording.as_ref())
                .and_then(|recording| recording.first_release_date.clone())
                .unwrap_or_default();
            let original_year = original_date.get(0..4).unwrap_or_default().to_string();
            vorbis.set("ORIGINALDATE", vec![original_date]);
            vorbis.set("ORIGINALYEAR", vec![original_year]);

            vorbis.set("BARCODE", vec![release.barcode.clone().unwrap_or_default()]);
            vorbis.set(
                "RELEASECOUNTRY",
                vec![release.country.clone().unwrap_or_default()],
            );
            vorbis.set(
                "RELEASESTATUS",
                vec![release.status.clone().unwrap_or_default()],
            );

            if let Some(media_list) = release.media.as_ref() {
                let total_discs = media_list.len();
                vorbis.set("TOTALDISCS", vec![total_discs.to_string()]);
                vorbis.set("DISCTOTAL", vec![total_discs.to_string()]);
                if let Some(track_count) = media_list.first().and_then(|media| media.track_count) {
                    vorbis.set("TOTALTRACKS", vec![track_count.to_string()]);
                    vorbis.set("TRACKTOTAL", vec![track_count.to_string()]);
                };
            }

            vorbis.set(
                "MEDIA",
                vec![
                    release
                        .media
                        .as_ref()
                        .and_then(|all_media| all_media.first())
                        .and_then(|media| media.format.clone())
                        .unwrap_or_default(),
                ],
            );
            vorbis.set("DISCNUMBER", vec![1.to_string()]);
        }
        dbg!(&tag);
        tag.write_to_path(&flac_filename).unwrap();
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
