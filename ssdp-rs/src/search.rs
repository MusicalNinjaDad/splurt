//! An SSDP searcher, that sends M-SEARCH and receives NOTIFY responses
//!
//! ## Example usage
//! ```
//! use ssdp_rs::search::Searcher;
//!
//! // Create a new searcher
//! let searcher = Searcher::new();
//!
//! // run a search - can call next().await on the result
//! // let some_sort_of_iterable_or_stream = searcher.search().await;
//! ```

use std::{
    io,
    net::{Ipv4Addr, SocketAddrV4},
};

use crate::udp::UdpStream;

pub struct Searcher {
    stream: UdpStream,
}

impl Searcher {
    pub fn new() -> io::Result<Self> {
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 1900).into();
        Ok(Searcher {
            stream: UdpStream::bind(addr)?,
        })
    }
}
