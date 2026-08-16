use clap::Parser;

#[derive(Parser)]
#[command(version)]
/// Rip CD audio tracks to WAV files
pub struct Rip {
    /// The drive path (e.g., E:)
    pub drive: String,
    /// The track number (1-indexed, or 0 for all tracks). If not provided, you'll be prompted.
    #[arg(default_value = None)]
    pub track_number: Option<usize>,
    /// Output filename (default: Track<n>.wav or All Tracks.wav for all)
    #[arg(long)]
    pub output: Option<String>,
    /// Non-interactive mode: use the latest release for CDs with multiple releases
    #[arg(short = 'n', long)]
    pub non_interactive: bool,
}

impl Rip {
    pub fn output_filename(&self, track_name: String) -> String {
        if let Some(ref filename) = self.output {
            return filename.clone();
        }
        let nn = self.track_number.unwrap_or(0);
        if nn == 0 {
            format!("All Tracks - {track_name}.wav")
        } else {
            format!("{nn:02} {track_name}.wav")
        }
    }
}
