#![cfg_attr(all(test, unstable_assert_matches), feature(assert_matches))]
#![cfg_attr(unstable_bool_to_result, feature(bool_to_result))]
#![cfg_attr(unstable_let_chains, feature(let_chains))]

//! A runtime-agnostic (known to work with tokio, futures-rs and futures-net executors) async ssdp
//! library. Including improved async UDP primitives: UDPListener & UDPStream.

pub mod message;
pub mod udp;
