pub mod albums;

use std::path::PathBuf;

/// Load a hex file and parse it as raw bytes
pub fn load_hex_file(path: &PathBuf) -> Vec<u8> {
    let content = std::fs::read_to_string(path).unwrap();
    crate::hex::hex_to_bytes(&content).unwrap()
}
