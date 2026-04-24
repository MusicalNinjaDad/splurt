#![cfg_attr(unstable_let_chains, feature(let_chains))]
#![feature(never_type)]
#![feature(try_blocks_heterogeneous)]
#![feature(try_trait_v2)]
#![feature(try_trait_v2_residual)]

use std::{
    collections::HashMap,
    fmt::Debug,
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    process::Termination as _T,
};

use clap::Parser;
use cotton_netif::get_interfaces;
use cotton_ssdp::{Advertisement, AsyncService, Notification};
use exit_safely::Termination;
use futures_util::StreamExt;
use try_v2::{Try, Try_ConvertResult};
use uuid::Uuid;

use ssdp_rs::udp::{UdpListener, UdpStream};

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
            let multicast = Ipv4Addr::new(239, 255, 255, 250);

            let std::net::IpAddr::V4(interface) = get_bind_addr()?.ip() else {
                todo!()
            };

            println!("will join multicast on interface {interface}");

            let loopback = Ipv4Addr::new(127, 0, 0, 1);
            let listen_addr = SocketAddrV4::new(loopback, 0).into();
            let mut listener = UdpListener::bind(&listen_addr).expect("sender");
            listener.join_multicast_v4(&multicast, &interface)?;

            let send_addr = listener.local_addr().expect("bound port");
            println!("listening on {:?}", send_addr);

            let listen_loop = async move {
                let mut incoming = [b'\x00'; 1024];
                try bikeshed Exit<()> {
                    loop {
                        println!("listening ...");
                        let (bytes, sent_by) = listener.recv_from(&mut incoming).await?;
                        println!(
                            "received: {} from {} ({} bytes)",
                            String::from_utf8_lossy(&incoming),
                            sent_by,
                            bytes
                        );
                    }
                }
            };

            futures::executor::block_on(listen_loop)?;
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

// Adapted from https://github.com/jakobhellermann/ssdp-client/blob/main/src/search.rs
fn get_bind_addr() -> Result<SocketAddr, std::io::Error> {
    // Windows 10 is multihomed so that the address that is used for the broadcast send is not guaranteed to be your local ip address, it can be any of the virtual interfaces instead.
    // Thanks to @dheijl for figuring this out <3 (https://github.com/jakobhellermann/ssdp-client/issues/3#issuecomment-687098826)
    let googledns: SocketAddr = ([8, 8, 8, 8], 80).into();
    let stream = UdpStream::new(&googledns)?;
    let bind_addr = stream.local_addr()?;

    Ok(bind_addr)
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
