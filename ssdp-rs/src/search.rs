//! An SSDP searcher, that sends M-SEARCH and receives NOTIFY responses
//!
//! ## Example usage
//! ```no_run
//! # // no reply possible so not run as test to avoid endless loop
//! # use std::io;
//! use ssdp_rs::search::Searcher;
//!
//! # fn main() -> io::Result<()> {
//! // Create a new searcher
//! let mut searcher = Searcher::new("splurt", "0.0.1", "splurt ssdp message repeater")?;
//!
//! // run a search - can call next().await on the result
//! # futures::executor::block_on( async {
//! searcher.search().await.expect("search executed");
//! # });
//!
//! // get the results
//! # futures::executor::block_on( async {
//! loop {
//!     let answer = searcher.next().await;
//!     // do something with answer
//! }
//! # });
//! # Ok(())
//! # }
//! ```

use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
};

use futures::{FutureExt, SinkExt, Stream, StreamExt};
use uuid::Uuid;

use crate::{
    message::Message,
    udp::{EventedUdpSocket, UdpSink, UdpStream},
};

const MULTICAST_IP: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const SSDP_PORT: u16 = 1900;
const MUTLICAST_SOCKET: SocketAddr = SocketAddr::new(IpAddr::V4(MULTICAST_IP), SSDP_PORT);
const MAX_MSG_SIZE: usize = 1024;

#[derive(Debug)]
pub struct Searcher {
    outgoing: UdpSink,
    mx: u8,
    os: String,
    os_version: String,
    product_name: String,
    product_version: String,
    friendly_name: String,
    uuid: Uuid,
}

#[derive(Debug)]
pub struct Listener {
    incoming: UdpStream<MAX_MSG_SIZE>,
}

impl Listener {
    pub fn new(addr: Ipv4Addr) -> io::Result<Self> {
        let addr = SocketAddrV4::new(addr, SSDP_PORT).into();
        let mut incoming = UdpStream::bind(addr)?;
        let IpAddr::V4(interface) = incoming.local_addr()?.ip() else {
            unimplemented!("no IPv6 support")
        };
        incoming
            .as_socket_mut()
            .join_multicast_v4(&MULTICAST_IP, &interface)?;
        Ok(Self { incoming })
    }
}

impl Stream for Listener {
    type Item = io::Result<(String, SocketAddr)>;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.as_mut()
            .incoming
            .poll_next_unpin(cx)
            .map_ok(|(msg, len, addr)| (String::from_utf8_lossy(&msg[..len]).to_string(), addr))
    }
}

impl Searcher {
    pub fn new(product_name: &str, product_version: &str, friendly_name: &str) -> io::Result<Self> {
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 1900).into();
        let uuid = Uuid::new_v4();
        let os_info = osinfo::get();
        let os = os_info.get_name();
        let os_version = os_info.get_version().to_string();

        Ok(Searcher {
            outgoing: UdpSink::bind(addr)?,
            mx: 5,
            os,
            os_version,
            product_name: product_name.to_string(),
            product_version: product_version.into(),
            friendly_name: friendly_name.to_string(),
            uuid,
        })
    }

    pub fn search<'s>(&'s mut self) -> Search<'s> {
        let Searcher {
            outgoing,
            mx,
            os,
            os_version,
            product_name,
            product_version,
            friendly_name,
            uuid,
        } = self;
        let msg = Message::new_search(
            *mx,
            os,
            os_version,
            product_name,
            product_version,
            friendly_name,
            *uuid,
        )
        .to_string();
        Search { outgoing, msg }
    }
}

/// The future returned by [Searcher::search]
pub struct Search<'searcher> {
    outgoing: &'searcher mut UdpSink,
    msg: String,
}

impl<'searcher> Future for Search<'searcher> {
    type Output = io::Result<()>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = &mut *self;
        let sink = &mut *this.outgoing;
        let msg = this.msg.as_bytes();
        sink.send((msg, &MUTLICAST_SOCKET))
            .poll_unpin(cx)
            .map_ok(|_| ())
    }
}
