use clap::Parser;
use redbook::{grab_track, into_wav};
use std::{
    fs::File,
    io::Write,
    process,
};

mod cli;
use cli::Rip;

fn main() {
    let rip = Rip::parse();

    let drive = &rip.drive;
    let track_number = rip.track_number;
    let output_filename = rip.output_filename();

    match grab_track(drive, track_number) {
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
        Ok(pcm) => {
            let mut dump = File::create_new(&output_filename).unwrap();
            let wav = into_wav(pcm);
            dump.write_all(&wav).unwrap();
            println!("Track {} ripped to {}", track_number, output_filename);
        }
    }
}
