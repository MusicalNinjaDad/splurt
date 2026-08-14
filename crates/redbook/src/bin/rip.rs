use redbook::grab_track;
use std::{
    env,
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
        Ok(data) => match args.get(2) {
            Some(_filename) => {
                todo!("dump to file");
            }
            None => {
                dbg!(data.len());
                let _ = io::stdout().write_all(&data);
            }
        },
    }
    println!("All OK - Press Enter to exit...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
}
