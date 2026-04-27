#![cfg_attr(unstable_let_chains, feature(let_chains))]
#![feature(future_join)]
#![feature(never_type)]
#![feature(try_blocks_heterogeneous)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use std::{
    collections::HashMap, fmt::Debug, future::join, io, net::Ipv4Addr, process::Termination as _T,
};

use clap::Parser;
use cotton_netif::get_interfaces;
use cotton_ssdp::{Advertisement, AsyncService, Notification};
use exit_safely::Termination;
use futures::StreamExt;
use try_v2::{Try, Try_ConvertResult};
use uuid::Uuid;

use ssdp_rs::search::{Listener, Searcher};

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

fn main() -> Exit<()> {
    let splurt = Splurt::try_parse()?;

    match &splurt.command {
        Command::Listen => {
            let mut searcher = Searcher::new(
                Ipv4Addr::UNSPECIFIED,
                "splurt",
                "v0.0.1",
                "splurt ssdp repeater",
            )?;
            let search = async {
                try bikeshed Exit<()> {
                    println!("sending an M-SEARCH");
                    searcher.search().await?
                }
            };

            let mut listener = Listener::new(Ipv4Addr::UNSPECIFIED)?;
            let listen_loop = async {
                try bikeshed Exit<()> {
                    loop {
                        println!("listening ...");
                        let (msg, sent_by) = listener.next().await.expect("a message")?;
                        println!("received: {} from {}", msg, sent_by);
                    }
                }
            };

            let both = join!(search, listen_loop);
            let (search, listen) = futures::executor::block_on(both);
            search?;
            listen?;

            // let run_both = async {
            //     try bikeshed Exit<()> {
            //         futures::future::join(listen_loop?, search?).await;
            //     }
            // };
            // // error[E0277]: the `?` operator can only be applied to values that implement `std::ops::Try`
            // // --> src/main.rs:73:43
            // // |
            // // 73 |                     futures::future::join(listen_loop?, search?).await;
            // // |                                           ^^^^^^^^^^^^ the `?` operator cannot be applied to type `{async block@src/main.rs:61:31: 61:36}`
            // // |
            // // = help: the nightly-only, unstable trait `std::ops::Try` is not implemented for `{async block@src/main.rs:61:31: 61:36}`
            // // help: consider `await`ing on the `Future`
            // // |
            // // 73 |                     futures::future::join(listen_loop.await?, search?).await;
            // // |

            // futures::executor::block_on(search)?;
            // futures::executor::block_on(listen_loop)?;
        }

        Command::Interfaces => {
            for e in get_interfaces()? {
                println!("{:?}", e);
            }
        }
        Command::Ssdp => {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime");
            let mut known_services = KnownServices::new();

            let ssdp_loop = async {
                try bikeshed Exit<()> {
                    let mut netif = cotton_netif::get_interfaces_async()?;
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
                                        let service = KnownService::from_notification(notification).expect("This is an alive");
                                        match known_services.insert(unique_service_name.clone(), service.clone()) {
                                            None => {
                                                println!("+  {notification_type}");
                                                println!("   {unique_service_name} at {location}");
                                            }
                                            Some(previous) if previous != service => {
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
                                                    println!(" - {}", previous.service_type);
                                                } else {
                                                    println!("!- {} -> {}", previous.service_type, notification_type);
                                                }
                                                println!("   {unique_service_name} at {}", previous.location);
                                            },
                                        }
                                    }
                                    None => {
                                        println!("SSDP listener closed");
                                        break
                                    }
                                }
                            },
                            event = netif.next() => {
                                match event {
                                    Some(event) => ssdp.on_network_event(&event?)?,
                                    None => {
                                        println!("Network interface monitor closed");
                                        break
                                    }
                                }
                            }
                        }
                    }
                }
            };
            rt.block_on(ssdp_loop)?;
        }
        Command::Test => {
            let uuid = Uuid::new_v4();
            let test_service = Advertisement {
                notification_type: "test".to_string(),
                location: "http://127.0.0.1:3333/test".to_string(),
            };

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("tokio runtime");

            let keep_alive = async {
                try bikeshed Exit<()> {
                    let mut netif = cotton_netif::get_interfaces_async()?;
                    let mut ssdp = AsyncService::new()?;
                    println!("advertising with uuid {}", uuid);
                    ssdp.advertise(uuid.to_string(), test_service);
                    loop {
                        match netif.next().await {
                            Some(event) => ssdp.on_network_event(&event?)?,
                            None => {
                                println!("Network inteface monitor closed");
                                break;
                            }
                        };
                    }
                }
            };

            rt.block_on(keep_alive)?;
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

impl<T: _T> From<futures_util::task::SpawnError> for Exit<T> {
    fn from(e: futures_util::task::SpawnError) -> Self {
        Self::Error(e.to_string())
    }
}
