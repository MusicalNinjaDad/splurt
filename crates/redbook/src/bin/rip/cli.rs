use clap::Parser;

#[derive(Parser)]
#[command(version)]
/// Rip CD audio tracks to WAV files
pub struct Rip {
    /// The drive path (e.g., E:)
    pub drive: String,
    /// The track number (1-indexed)
    pub track_number: usize,
    /// Output filename (default: Track<n>.wav)
    #[arg(long)]
    pub output: Option<String>,
    /// Use the latest release (non-interactive mode for CDs with multiple releases)
    #[arg(long)]
    pub latest: bool,
}

impl Rip {
    pub fn output_filename(&self, track_name: String) -> String {
        if let Some(ref filename) = self.output {
            return filename.clone();
        }
        format!("{nn:02} {track_name}.wav", nn = self.track_number)
    }
}
