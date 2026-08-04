use redbook::read_toc;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let drive = &args[1];
    if let Err(e) = read_toc(drive) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
