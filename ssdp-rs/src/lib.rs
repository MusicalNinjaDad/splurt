#![cfg_attr(all(test, unstable_assert_matches), feature(assert_matches))]
#![cfg_attr(unstable_bool_to_result, feature(bool_to_result))]
#![cfg_attr(unstable_let_chains, feature(let_chains))]
#![cfg_attr(unstable_never_type, feature(never_type))]

//! A runtime-agnostic (known to work with tokio, futures-rs and futures-net executors) async ssdp
//! library. Including improved async UDP primitives: UDPListener & UDPStream.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub mod listener;
pub mod message;
pub mod search;
pub mod udp;

pub use listener::Listener;
pub use search::Searcher;

pub const MULTICAST_IP: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
pub const SSDP_PORT: u16 = 1900;
pub const MULTICAST: SocketAddr = SocketAddr::new(IpAddr::V4(MULTICAST_IP), SSDP_PORT);
pub const MAX_MSG_SIZE: usize = 1024;
