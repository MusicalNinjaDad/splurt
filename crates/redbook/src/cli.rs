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
}

impl Rip {
    pub fn output_filename(&self) -> String {
        self.output.clone().unwrap_or_else(|| format!("Track{}.wav", self.track_number))
    }
}
