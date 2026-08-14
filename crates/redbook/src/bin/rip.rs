use redbook::{grab_track, into_wav};
use std::{
    env,
    fs::File,
    io::{self, Write},
};

fn main() {
    let args: Vec<String> = env::args().collect();
    let default_drive = "E:".to_string();
    let drive = args.get(1).unwrap_or(&default_drive);
    match grab_track(drive) {
        Err(e) => {
            eprintln!("Error: {}", e);

            println!("Press Enter to exit...");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();

            std::process::exit(1);
        }
        Ok(pcm) => match args.get(2) {
            Some(filename) => {
                let mut dump = File::create_new(filename).unwrap();
                let wav = into_wav(pcm);
                dbg!(dump.write(&wav));
            }
            None => {
                dbg!(pcm.len());
            }
        },
    }
    println!("All OK - Press Enter to exit...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
}
