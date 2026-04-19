#![cfg_attr(unstable_let_chains, feature(let_chains))]
#![feature(never_type)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use std::{collections::HashMap, fmt::Debug, io, process::Termination as _T};

use clap::Parser;
use cotton_netif::get_interfaces;
use cotton_ssdp::{Advertisement, AsyncService, Notification};
use exit_safely::Termination;
use futures_util::StreamExt;
use try_v2::{Try, Try_ConvertResult};
use uuid::Uuid;

mod cli;
use cli::*;

#[derive(Debug, Clone, PartialEq, PartialOrd)]
struct KnownService {
    service_type: String,
    location: String,
}
type UniqueServiceName = String;

type KnownServices = HashMap<UniqueServiceName, KnownService>;

impl KnownService {
    fn from_notification(notification: &Notification) -> Option<Self> {
        match notification {
            Notification::Alive {
                notification_type: service_type,
                unique_service_name: _,
                location,
            } => Some(Self {
                service_type: service_type.clone(),
                location: location.clone(),
            }),
            Notification::ByeBye { .. } => None,
        }
    }
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
        //TODO: This is directly from the cotton example and needs a bit of rework.
        //      See #5 & #6
        Command::Ssdp => {
            let mut netif = cotton_netif::get_interfaces_async()?;
            let mut known_services = KnownServices::new();
            let mut ssdp = AsyncService::new()?;
            let mut stream = ssdp.subscribe("ssdp:all");
            loop {
                tokio::select! {
                    notification = stream.next() => {
                        match notification {
                            Some(ref notification @ Notification::Alive {
                                ref notification_type,
                                ref unique_service_name,
                                ref location,
                            }) => {
                                match known_services.insert(unique_service_name.clone(), KnownService::from_notification(notification).expect("This is an alive")) {
                                    None => {
                                        println!("+  {notification_type}");
                                        println!("   {unique_service_name} at {location}");
                                    }
                                    Some(previous) if previous != KnownService::from_notification(notification).expect("This is an alive") => {
                                        println!("!  {} -> {}", previous.service_type, notification_type);
                                        println!("   {unique_service_name} at {} -> {}", previous.location, location);
                                    },
                                    Some(_) => (),
                                }
                            }
                            Some(Notification::ByeBye{ notification_type, unique_service_name }) => {
                                match known_services.remove_entry(&unique_service_name) {
                                    None =>{
                                        println!("+- {notification_type}");
                                        println!("   {unique_service_name} at unknown");
                                    },
                                    Some((_, previous)) => {
                                        if previous.service_type == notification_type {
                                            println!("!- {} -> {}", previous.service_type, notification_type);
                                        } else {
                                            println!(" - {}", previous.service_type);
                                        }
                                        println!("   {unique_service_name} at {}", previous.location);
                                    },
                                }
                            }
                            None => {
                                println!("SSDP listener closed");
                                break
                            },
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
