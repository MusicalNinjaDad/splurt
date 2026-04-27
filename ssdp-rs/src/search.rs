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

use futures::{FutureExt, SinkExt, StreamExt};
use uuid::Uuid;

use crate::{
    message::Message,
    udp::{EventedUdpSocket, UdpSink, UdpStream},
};

const MULTICAST_IP: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const MUTLICAST_SOCKET: SocketAddr = SocketAddr::new(IpAddr::V4(MULTICAST_IP), 1900);

#[derive(Debug)]
pub struct Searcher {
    incoming: UdpStream<512>,
    outgoing: UdpSink,
    mx: u8,
    os: String,
    os_version: String,
    product_name: String,
    product_version: String,
    friendly_name: String,
    uuid: Uuid,
}

impl Searcher {
    pub fn new(product_name: &str, product_version: &str, friendly_name: &str) -> io::Result<Self> {
        let addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 1900).into();
        let uuid = Uuid::new_v4();
        let os_info = osinfo::get();
        let os = os_info.get_name();
        let os_version = os_info.get_version().to_string();
        let mut incoming = UdpStream::bind(addr)?;
        let IpAddr::V4(interface) = incoming.local_addr()?.ip() else {
            unimplemented!("no IPv6 support")
        };
        incoming
            .as_socket_mut()
            .join_multicast_v4(&MULTICAST_IP, &interface)?;
        Ok(Searcher {
            incoming,
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
            incoming: _,
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

    pub async fn next(&mut self) -> Option<io::Result<(String, SocketAddr)>> {
        match self.incoming.next().await? {
            Ok((msg, len, sender)) => {
                let msg = String::from_utf8_lossy(&msg[..len]).to_string();
                Some(Ok((msg, sender)))
            }
            Err(e) => Some(Err(e)),
        }
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
