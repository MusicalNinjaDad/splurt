use std::{fs, process::Termination as _T};

use tracing_subscriber::{
    filter::LevelFilter, fmt::layer, prelude::*, registry, util::TryInitError,
};

use crate::{Exit, Rip, cli::LogLevel};

impl Rip {
    pub fn init_tracing(&self) -> Exit<()> {
        let stdout_level = match (self.verbose, self.quiet) {
            // Clap ensures mutual exclusivity
            (0, 0) => LevelFilter::INFO,
            (1, _) => LevelFilter::DEBUG,
            (2.., _) => LevelFilter::TRACE,
            (_, 1..) => LevelFilter::OFF,
        };
        let stdout = layer().with_filter(stdout_level);

        let stderr_level = match self.quiet {
            2.. => LevelFilter::OFF,
            _ => LevelFilter::WARN,
        };
        let stderr = layer()
            .with_writer(std::io::stderr)
            .with_filter(stderr_level);

        // If let Some to be explicit about side-effects (file creation)
        let file = if let Some(logpath) = &self.log {
            let file = fs::File::options().append(true).open(logpath)?;
            let loglevel = LevelFilter::from(&self.loglevel);
            let json = match self.format {
                crate::cli::LogFormat::Human => None,
                crate::cli::LogFormat::Json => Some(layer().json()),
            };
            Some(
                layer()
                    .with_writer(file)
                    .with_filter(loglevel)
                    .and_then(json),
            )
        } else {
            None
        };

        registry().with(stdout).with(stderr).with(file).try_init()?;

        Exit::Ok(())
    }
}

impl From<&LogLevel> for LevelFilter {
    fn from(level: &LogLevel) -> Self {
        match level {
            LogLevel::Warn => LevelFilter::WARN,
            LogLevel::Info => LevelFilter::INFO,
            LogLevel::Debug => LevelFilter::DEBUG,
            LogLevel::Trace => LevelFilter::TRACE,
        }
    }
}

impl<T: _T> From<TryInitError> for Exit<T> {
    fn from(error: TryInitError) -> Self {
        Self::Logging(error.to_string())
    }
}
