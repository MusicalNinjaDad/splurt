use clap::Parser;

#[derive(Parser)]
#[command(version)]
/// Read flac tags
pub struct Tag {
    /// The file path (e.g., music.flac)
    pub filename: String,
}
