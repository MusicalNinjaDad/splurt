//! Test parsing TOC for "Definitely Maybe"
//! https://musicbrainz.org/release/9822581d-98bf-3f97-a94c-4b1350d090aa

use std::path::{Path, PathBuf};

use cdtoc::Toc;
use redbook::{Frame, Msf};
use windows_sys::Win32::Devices::Cdrom::CDROM_TOC;

/// Load a hex file and parse it as raw bytes
fn load_hex_file<P: AsRef<Path>>(path: &P) -> Vec<u8> {
    let content = std::fs::read_to_string(path).expect("Failed to read file");
    hex_to_bytes(&content)
}

/// Convert hex in form `00 01 02` to bytes
fn hex_to_bytes(hex: &str) -> Vec<u8> {
    // Remove all non-hex characters (spaces, newlines, etc.)
    let hex_str: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect();

    // Convert hex string to bytes
    (0..hex_str.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16).expect("Invalid hex digit"))
        .collect()
}

unsafe fn load_cdrom_toc(bytes: Vec<u8>) -> CDROM_TOC {
    unsafe { *(bytes.as_ptr() as *const _) }
}

struct TocEntry {
    track: u8,
    start: Msf,
}

impl TocEntry {
    fn from_bytes(data: &[u8]) -> Self {
        let track = data[3];
        let start = Msf::new(data[8] as i8, data[9] as i8, data[10] as i8);
        Self { track, start }
    }
}

/// Converts a hex dump of raw TOC data to the format
/// `[audio trackcount]+[first audio track address]+[second audio track address]`
/// as used by [cdtoc::Toc::from_cdtoc] and described at
/// https://forum.dbpoweramp.com/forum/other-topics/developers-corner/16082-flac-ogg-vorbis-storage-of-cdtoc?16705-FLAC-amp-Ogg-Vorbis-Storage-of-CDTOC=&s=3ca0c65ee58fc45489103bb1c39bfac0&viewfull=1#post76686
fn parse_toc(bytes: Vec<u8>) -> String {
    let mut entries: Vec<_> = bytes
        .chunks_exact(11)
        .map(TocEntry::from_bytes)
        .filter(|entry| entry.track != 0xA0 && entry.track != 0xA1)
        .collect();
    entries.sort_by_key(|entry| entry.track);
    let tracks = entries.len() - 1; // Special entry A2 (leadout)
    let timings = entries
        .iter()
        .map(|entry| {
            format!(
                "{frames:02x}+",
                frames = Frame::from(entry.start).as_usize()
            )
        })
        .collect::<String>();
    format!("{tracks:02x}+{timings}")
        .trim_end_matches("+")
        .to_string()
}

#[test]
fn cdrom_toc() {
    let path = PathBuf::from("tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/CDROM_TOC.hex");
    let toc_dump = load_hex_file(&path);
    let toc = unsafe { load_cdrom_toc(toc_dump) };
    assert_eq!(toc.FirstTrack, 1);
    assert_eq!(toc.LastTrack, 11);
    assert_eq!(toc.TrackData[0].Address, [0, 0, 0, 33])
}

#[test]
fn load_toc() {
    let path = PathBuf::from("tests/assets/9822581d-98bf-3f97-a94c-4b1350d090aa/TOC.hex");
    let toc_dump = load_hex_file(&path);
    let toc_string = parse_toc(toc_dump);
    dbg!(&toc_string);
    let toc = Toc::from_cdtoc(toc_string).unwrap();
    assert_eq!(toc.audio_len(), 11)
}
