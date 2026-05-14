use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version)]
/// Listen for and splurt out SSDP messages
pub struct Splurt {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// list interfaces
    Interfaces,
    /// list SSDP services
    Ssdp,
    /// advertise dummy service
    Test,
    /// listen to udp messages
    Listen,
    /// UI to silently listen (will not send out an initial search)
    Snoop,
}
