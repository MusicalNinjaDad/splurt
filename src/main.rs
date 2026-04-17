#![feature(never_type)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use std::{collections::HashMap, fmt::Debug, io, process::Termination as _T};

use clap::{Parser, Subcommand};
use cotton_netif::get_interfaces;
use cotton_ssdp::{AsyncService, Notification};
use exit_safely::Termination;
use futures_util::StreamExt;
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
    Interfaces,
    /// list SSDP services
    Ssdp,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Exit<()> {
    let splurt = Splurt::try_parse()?;

    match &splurt.command {
        Command::Interfaces => {
            for e in get_interfaces()? {
                println!("{:?}", e);
            }
        }
        Command::Ssdp => {
            let mut netif = cotton_netif::get_interfaces_async()?;
            let mut ssdp = AsyncService::new()?;
            let mut map = HashMap::new();
            let mut stream = ssdp.subscribe("ssdp:all");
            loop {
                tokio::select! {
                    notification = stream.next() => {
                        if let Some(r) = notification {
                            if let Notification::Alive {
                                ref notification_type,
                                ref unique_service_name,
                                ref location,
                            } = r
                            {
                                if !map.contains_key(unique_service_name) {
                                    println!("+ {notification_type}");
                                    println!("  {unique_service_name} at {location}");
                                    map.insert(unique_service_name.clone(), r);
                                }
                            }
                        }
                    },
                    e = netif.next() => {
                        if let Some(Ok(event)) = e {
                            ssdp.on_network_event(&event)?;
                        }
                    }
                }
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

impl<T: _T> From<cotton_ssdp::udp::Error> for Exit<T> {
    fn from(e: cotton_ssdp::udp::Error) -> Self {
        Self::Error(e.to_string())
    }
}
