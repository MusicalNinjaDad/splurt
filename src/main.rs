#![cfg_attr(unstable_let_chains, feature(let_chains))]
#![feature(never_type)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use std::{
    collections::HashMap,
    fmt::Debug,
    io::{self, stdout},
    process::Termination as _T,
};

use clap::{CommandFactory, Parser};
use cotton_netif::get_interfaces;
use cotton_ssdp::{Advertisement, AsyncService, Notification};
use exit_safely::Termination;
use futures_util::StreamExt;
use try_v2::{Try, Try_ConvertResult};
use uuid::Uuid;

mod cli;
use cli::*;

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
            let mut known_services = HashMap::<String, Notification>::new();
            let mut ssdp = AsyncService::new()?;
            let mut stream = ssdp.subscribe("ssdp:all");
            loop {
                tokio::select! {
                    notification = stream.next() => {
                        if let Some(Notification::Alive {
                                ref notification_type,
                                ref unique_service_name,
                                ref location,
                            }) = notification
                            && !known_services.contains_key(unique_service_name)
                            {
                                println!("+ {notification_type}");
                                println!("  {unique_service_name} at {location}");
                                known_services.insert(unique_service_name.clone(), notification.expect("inside if let Some"));
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
        Command::Test => {
            let mut netif = cotton_netif::get_interfaces_async()?;
            let mut ssdp = AsyncService::new()?;
            let uuid = Uuid::new_v4();
            let test_service = Advertisement {
                notification_type: "test".to_string(),
                location: "http://127.0.0.1:3333/test".to_string(),
            };
            println!("advertising with uuid {}", uuid);
            ssdp.advertise(uuid.to_string(), test_service);
            loop {
                let event = netif.next().await;
                if let Some(event) = event {
                    ssdp.on_network_event(&event?)?;
                }
            }
        }
        Command::Man => {
            let manpage = clap_mangen::Man::new(Splurt::command());
            manpage.render(&mut stdout())?;
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
