use std::path::PathBuf;

use clap::Parser;

#[derive(Parser)]
#[command(version)]
/// Rip CD audio tracks to WAV files
pub struct Rip {
    /// The drive path (e.g., E:)
    pub drive: String,
    /// The track number (1-indexed)
    #[arg(default_value = None)]
    pub track_number: Option<usize>,
    /// Rip all tracks
    #[arg(short = 'a', long, conflicts_with = "track_number")]
    pub all: bool,
    /// Non-interactive mode: use the latest release for CDs with multiple releases
    #[arg(short = 'n', long)]
    pub non_interactive: bool,
    // Tracing verbosity flags
    #[arg(short = 'v', action = clap::ArgAction::Count)]
    pub verbose: u8,
    #[arg(short = 'q', action = clap::ArgAction::Count)]
    pub quiet: u8,
    #[arg(long, value_name = "LOGFILE")]
    pub debug: Option<PathBuf>,
    #[arg(long, value_name = "LOGFILE")]
    pub trace: Option<PathBuf>,
    #[arg(long)]
    pub json: bool,
}
