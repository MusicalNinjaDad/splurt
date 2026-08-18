use redbook::hex::hex_dump;
use redbook::win::CdDrive;

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let drive = CdDrive::open(&args[1]).unwrap();
    let toc = drive.toc_as_raw_bytes();
    println!("{}", hex_dump(toc));
}
