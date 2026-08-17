#![feature(never_type)]
#![feature(try_blocks)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use clap::Parser;
use exit_safely::Termination;
use flacenc::{component::BitRepr, error::Verify};
use metaflac::Tag;
use redbook::{AudioCd, AudioCdExt, RippedTrack, into_wav};
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
                let track_name = if let Some(release) = release {
                    release
                        .media
                        .as_ref()
                        .and_then(|all_media| all_media.first())
                        .and_then(|media| media.tracks.as_ref())
                        .and_then(|tracks| {
                            tracks
                                .iter()
                                .find(|trk| {
                                    trk.number.as_ref().and_then(|trk_num| trk_num.parse().ok())
                                        == Some(track.track_number)
                                })
                                .and_then(|trk| trk.title.as_ref())
                        })
                        .map(|title| title.as_str())
                        .unwrap_or("Unknown")
                } else {
                    "Unknown"
                };
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

    let mut cover = File::create_new("front.jpeg")?;
    match cd.disc_mut().cover_art() {
        Ok(Some(data)) => cover.write_all(data)?,
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
        let track_name = if let Some(release) = release {
            release
                .media
                .as_ref()
                .and_then(|all_media| all_media.first())
                .and_then(|media| media.tracks.as_ref())
                .and_then(|tracks| {
                    tracks
                        .iter()
                        .find(|trk| {
                            trk.number.as_ref().and_then(|trk_num| trk_num.parse().ok())
                                == Some(track.track_number)
                        })
                        .and_then(|trk| trk.title.as_ref())
                })
                .map(|title| title.as_str())
                .unwrap_or_default()
        } else {
            Default::default()
        };

        let output_filename =
            PathBuf::from([format!("{:02}", track.track_number), track_name.to_string()].join(" "));

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
        
        // Set all available metadata from the release
        if let Some(release) = cd.disc().release() {
            // Basic album information
            if !release.title.is_empty() {
                vorbis.set_album(vec![release.title.clone()]);
            }
            
            // Album artist information
            if let Some(artist_credits) = &release.artist_credit {
                if !artist_credits.is_empty() {
                    let album_artist = artist_credits.iter()
                        .map(|ac| ac.name.clone())
                        .collect::<Vec<String>>()
                        .join(", ");
                    vorbis.set_album_artist(vec![album_artist.clone()]);
                    vorbis.set_artist(vec![album_artist]);
                    
                    // Set album artist sort
                    if let Some(first_credit) = artist_credits.first() {
                        if let Some(artist) = &first_credit.artist {
                            if let Some(sort_name) = &artist.sort_name {
                                vorbis.set("ALBUMARTISTSORT", vec![sort_name.clone()]);
                            }
                        }
                    }
                    
                    // Set album artist ID
                    if let Some(first_credit) = artist_credits.first() {
                        if let Some(artist) = &first_credit.artist {
                            if let Some(artist_id) = &artist.id {
                                vorbis.set("MUSICBRAINZ_ALBUMARTISTID", vec![artist_id.clone()]);
                            }
                        }
                    }
                }
            }
            
            // Date information
            if let Some(date) = &release.date {
                vorbis.set("ORIGINALDATE", vec![date.clone()]);
                if date.len() >= 4 {
                    vorbis.set("ORIGINALYEAR", vec![date[..4].to_string()]);
                }
            }
            
            // Release group information
            if let Some(release_group) = &release.release_group {
                if let Some(rg_id) = &release_group.id {
                    vorbis.set("MUSICBRAINZ_RELEASEGROUPID", vec![rg_id.clone()]);
                }
                if let Some(rg_type) = &release_group.primary_type {
                    vorbis.set("RELEASETYPE", vec![rg_type.clone()]);
                } else if let Some(rg_type) = &release_group.r#type {
                    vorbis.set("RELEASETYPE", vec![rg_type.clone()]);
                }
            }
            
            // MusicBrainz album ID
            vorbis.set("MUSICBRAINZ_ALBUMID", vec![release.id.clone()]);
            
            // Release status
            if let Some(status) = &release.status {
                vorbis.set("RELEASESTATUS", vec![status.clone()]);
            }
            
            // Barcode
            if let Some(barcode) = &release.barcode {
                vorbis.set("BARCODE", vec![barcode.clone()]);
            }
            
            // Catalog number
            if let Some(label_info_list) = &release.label_info_list {
                for label_info in label_info_list {
                    if let Some(cat_num) = &label_info.catalog_number {
                        vorbis.set("CATALOGNUMBER", vec![cat_num.clone()]);
                        break; // Use first catalog number
                    }
                }
            }
            
            // Script from text representation
            if let Some(text_rep) = &release.text_representation {
                if let Some(script) = &text_rep.script {
                    vorbis.set("SCRIPT", vec![script.clone()]);
                }
            }
            
            // Country
            if let Some(country) = &release.country {
                vorbis.set("RELEASECOUNTRY", vec![country.clone()]);
            }
            
            // Media information
            if let Some(media_list) = &release.media {
                let total_discs = media_list.len() as u32;
                let total_tracks: u32 = media_list.iter()
                    .filter_map(|m| m.track_count)
                    .sum();
                
                vorbis.set("TOTALDISCS", vec![total_discs.to_string()]);
                vorbis.set("DISCTOTAL", vec![total_discs.to_string()]);
                
                if total_tracks > 0 {
                    vorbis.set("TOTALTRACKS", vec![total_tracks.to_string()]);
                    vorbis.set("TRACKTOTAL", vec![total_tracks.to_string()]);
                }
                
                // Media format
                if let Some(first_media) = media_list.first() {
                    if let Some(format) = &first_media.format {
                        vorbis.set("MEDIA", vec![format.clone()]);
                    }
                    // Disc number from media position
                    if let Some(position) = first_media.position {
                        vorbis.set("DISCNUMBER", vec![position.to_string()]);
                    }
                }
            }
            
            // Now handle track-specific information
            if let Some(media_list) = &release.media {
                if let Some(first_media) = media_list.first() {
                    if let Some(tracks) = &first_media.tracks {
                        if let Some(track_info) = tracks.iter().find(|t| {
                            t.number.as_ref().and_then(|n| n.parse().ok()) == Some(track.track_number)
                        }) {
                            // Track title
                            if let Some(title) = &track_info.title {
                                vorbis.set_title(vec![title.clone()]);
                            }
                            
                            // Track number
                            if let Some(track_num) = &track_info.number {
                                vorbis.set("TRACKNUMBER", vec![track_num.clone()]);
                            }
                            
                            // Track MusicBrainz ID
                            if let Some(track_id) = &track_info.id {
                                vorbis.set("MUSICBRAINZ_TRACKID", vec![track_id.clone()]);
                            }
                            
                            // Track artist information (if different from album artist)
                            if let Some(track_artist_credits) = &track_info.artist_credit {
                                if !track_artist_credits.is_empty() {
                                    let track_artist = track_artist_credits.iter()
                                        .map(|ac| ac.name.clone())
                                        .collect::<Vec<String>>()
                                        .join(", ");
                                    vorbis.set("ARTISTS", vec![track_artist.clone()]);
                                    
                                    // Track artist sort
                                    if let Some(first_credit) = track_artist_credits.first() {
                                        if let Some(artist) = &first_credit.artist {
                                            if let Some(sort_name) = &artist.sort_name {
                                                vorbis.set("ARTISTSORT", vec![sort_name.clone()]);
                                            }
                                            if let Some(artist_id) = &artist.id {
                                                vorbis.set("MUSICBRAINZ_ARTISTID", vec![artist_id.clone()]);
                                            }
                                        }
                                    }
                                }
                            }
                            
                            // Recording information
                            if let Some(recording) = &track_info.recording {
                                // Recording artist information
                                if let Some(recording_artist_credits) = &recording.artist_credit {
                                    if !recording_artist_credits.is_empty() {
                                        let recording_artist = recording_artist_credits.iter()
                                            .map(|ac| ac.name.clone())
                                            .collect::<Vec<String>>()
                                            .join(", ");
                                        // ARTIST tag for the track artist
                                        vorbis.set("ARTIST", vec![recording_artist]);
                                    }
                                }
                                
                                // MusicBrainz release track ID (recording ID)
                                vorbis.set("MUSICBRAINZ_RELEASETRACKID", vec![recording.id.clone()]);
                            }
                        }
                    }
                }
            }
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
