#![feature(never_type)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use std::{fmt::Debug, io, process::Termination as _T};

use clap::{Parser, Subcommand};
use cotton_netif::get_interfaces;
use exit_safely::Termination;
use try_v2::{Try, Try_ConvertResult};

#[derive(Parser)]
#[command(version)]
struct Splurt {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// list interfaces
    List,
}

fn main() -> Exit<()> {
    let splurt = Splurt::try_parse()?;

    match &splurt.command {
        Command::List => {
            for e in get_interfaces()? {
                println!("{:?}", e);
            }
        }
    }
    Exit::Ok(())
}

#[derive(Debug, Termination, Try, Try_ConvertResult, PartialEq, PartialOrd, Eq, Ord)]
#[repr(u8)]
#[must_use]
pub enum Exit<T: _T> {
    Ok(T) = 0,
    Error(String) = 1,
    InvocationError(String) = 2,
    IO(String) = 3,
}

impl<T: _T> From<clap::Error> for Exit<T> {
    fn from(e: clap::Error) -> Self {
        Self::InvocationError(e.to_string())
    }
}

impl<T: _T> From<io::Error> for Exit<T> {
    fn from(e: io::Error) -> Self {
        Self::IO(e.to_string())
    }
}
