#![cfg_attr(all(test, unstable_assert_matches), feature(assert_matches))]
#![cfg_attr(unstable_adt_const_params, feature(adt_const_params))]
#![cfg_attr(unstable_if_let_guard, feature(if_let_guard))]
#![cfg_attr(unstable_let_chains, feature(let_chains))]
#![cfg_attr(unstable_never_type, feature(never_type))]
#![feature(try_blocks_heterogeneous)]
#![allow(incomplete_features)]
#![feature(unsized_const_params)]

//! A runtime-agnostic (known to work with tokio, futures-rs and futures-net executors) async ssdp
//! library. Including improved async UDP primitives: UDPListener & UDPStream.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};

pub mod error;
pub mod listener;
pub mod message;
pub mod search;

pub use error::{Error, Result};
pub use listener::Listener;
pub use search::Searcher;

pub const MULTICAST_IP: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
pub const SSDP_PORT: u16 = 1900;
pub const MULTICAST: SocketAddr = SocketAddr::new(IpAddr::V4(MULTICAST_IP), SSDP_PORT);
pub const MAX_MSG_SIZE: usize = 1024;
