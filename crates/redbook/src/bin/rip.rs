use redbook::read_toc;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let default_drive = "E:".to_string();
    let drive = args.get(1).unwrap_or(&default_drive);
    if let Err(e) = read_toc(drive) {
        eprintln!("Error: {}", e);

        println!("Press Enter to exit...");
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();

        std::process::exit(1);
    }
    println!("All OK - Press Enter to exit...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).ok();
}
