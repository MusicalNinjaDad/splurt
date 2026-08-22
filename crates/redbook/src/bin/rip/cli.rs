use std::path::PathBuf;

use clap::{Parser, ValueEnum};

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
    /// Increase verbosity, can be provided multiple times (e.g. -vv) to be even more verbose.
    #[arg(short = 'v', action = clap::ArgAction::Count)]
    pub verbose: u8,
    /// Reduce verbosity, can be provided multiple times (e.g. -qq) to be even quieter.
    #[arg(short = 'q', action = clap::ArgAction::Count, conflicts_with = "verbose")]
    pub quiet: u8,
    /// Output information to <LOGFILE>. Not affected by -v / -q
    #[arg(long, value_name = "LOGFILE")]
    pub log: Option<PathBuf>,
    /// Information level to ouput to logfile
    #[arg(long, value_name = "LOGLEVEL", value_enum, default_value_os_t, requires = "log")]
    pub loglevel: LogLevel,
    /// Logfile format
    #[arg(long, value_name = "FORMAT", value_enum, default_value_os_t, requires = "log")]
    pub format: LogFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default)]
pub enum LogLevel {
    Warn,
    Info,
    #[default]
    Debug,
    Trace,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Default)]
pub enum LogFormat {
    #[default]
    Human,
    Json,
}