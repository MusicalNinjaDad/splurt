//! An SSDP searcher, that sends M-SEARCH and receives NOTIFY responses
//!
//! ## Example usage
//! ```no_run
//! # // no reply possible so not run as test to avoid endless loop
//! # use std::{io, net::Ipv4Addr};
//! use futures::StreamExt;
//! use ssdp_rs::search::{Listener, Searcher};
//!
//! # fn main() -> io::Result<()> {
//! // Create a new searcher
//! let mut searcher = Searcher::new(
//!     Ipv4Addr::UNSPECIFIED,
//!     "splurt",
//!     "v0.0.1",
//!     "splurt ssdp repeater",
//! )?;
//!
//! // run a search - can call next().await on the result
//! # futures::executor::block_on( async {
//! searcher.search().await.expect("search executed");
//! # });
//!
//! // listen for messages on your network
//! let mut listener = Listener::new(Ipv4Addr::UNSPECIFIED)?;
//! # futures::executor::block_on( async {
//! loop {
//!     let answer = listener.next().await;
//!     // do something with answer
//! }
//! # });
//! # Ok(())
//! # }
//! ```

use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
    time::Duration,
};

use futures::{SinkExt, Stream, StreamExt};
use futures_timer::Delay;
use uuid::Uuid;

use crate::{
    message::{Message, Mx},
    udp::{EventedUdpSocket, UdpSink, UdpStream},
};

const MULTICAST_IP: Ipv4Addr = Ipv4Addr::new(239, 255, 255, 250);
const SSDP_PORT: u16 = 1900;
const MUTLICAST: SocketAddr = SocketAddr::new(IpAddr::V4(MULTICAST_IP), SSDP_PORT);
const MAX_MSG_SIZE: usize = 1024;

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
#[derive(Debug)]
pub struct Searcher {
    outgoing: UdpSink,
    pub mx: Mx,
    os: String,
    os_version: String,
    product_name: String,
    product_version: String,
    friendly_name: String,
    uuid: Uuid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct UpnpBuilder<'a> {
    addr: Option<Ipv4Addr>,
    port: Option<u16>,
    ttl: Option<u32>,

    mx: Option<u8>,
    os: Option<&'a str>,
    os_version: Option<&'a str>,
    product_name: &'a str,
    product_version: &'a str,
    friendly_name: &'a str,
    uuid: Option<Uuid>,
}

impl UpnpBuilder<'_> {
    pub fn ip(&mut self, addr: Ipv4Addr) -> &mut Self {
        self.addr = Some(addr);
        self
    }

    pub fn port(&mut self, port: u16) -> &mut Self {
        self.port = Some(port);
        self
    }

    pub fn ttl(&mut self, ttl: u32) -> &mut Self {
        self.ttl = Some(ttl);
        self
    }

    pub fn mx(&mut self, mx: u8) -> &mut Self {
        self.mx = Some(mx);
        self
    }

    pub fn uuid(&mut self, uuid: Uuid) -> &mut Self {
        self.uuid = Some(uuid);
        self
    }

    pub fn build(&mut self) -> io::Result<Searcher> {
        let ip = self.addr.unwrap_or(Ipv4Addr::UNSPECIFIED);
        let port = self.port.unwrap_or(SSDP_PORT);
        let addr = SocketAddrV4::new(ip, port).into();
        let ttl = self.ttl.unwrap_or(2);
        let mut outgoing = UdpSink::bind(addr)?;
        outgoing.as_socket_mut().set_ttl(ttl)?;

        let mx = Mx::try_from(self.mx.unwrap_or(5))?;
        let os_info = osinfo::get();
        let os = os_info.get_name();
        let os_version = os_info.get_version().to_string();
        let product_name = self.product_name.to_string();
        let product_version = self.product_version.to_string();
        let friendly_name = self.friendly_name.to_string();
        let uuid = self.uuid.unwrap_or(Uuid::new_v4());
        Ok(Searcher {
            outgoing,
            mx,
            os,
            os_version,
            product_name,
            product_version,
            friendly_name,
            uuid,
        })
    }
}

/// Create a new Searcher with following defaults (can be later changed):
///
/// - MX: 5 (max as per spec)
/// - TTL: 2 (recommended by spec)
impl Searcher {
    pub fn new(
        addr: Ipv4Addr,
        product_name: &str,
        product_version: &str,
        friendly_name: &str,
    ) -> io::Result<Self> {
        let addr = SocketAddrV4::new(addr, 1900).into();
        let uuid = Uuid::new_v4();
        let os_info = osinfo::get();
        let os = os_info.get_name();
        let os_version = os_info.get_version().to_string();
        let mut outgoing = UdpSink::bind(addr)?;
        outgoing.as_socket_mut().set_ttl(2)?;
        Ok(Searcher {
            outgoing,
            mx: Mx::try_from(5)?,
            os,
            os_version,
            product_name: product_name.to_string(),
            product_version: product_version.into(),
            friendly_name: friendly_name.to_string(),
            uuid,
        })
    }

    pub fn custom<'a>(
        product_name: &'a str,
        product_version: &'a str,
        friendly_name: &'a str,
    ) -> UpnpBuilder<'a> {
        UpnpBuilder {
            product_name,
            product_version,
            friendly_name,
            ..Default::default()
        }
    }

    pub fn ttl(&self) -> io::Result<u32> {
        self.outgoing.as_socket().ttl()
    }

    pub fn set_ttl(&mut self, ttl: u32) -> io::Result<u32> {
        let current = self.ttl()?;
        self.outgoing.as_socket_mut().set_ttl(ttl).map(|_| current)
    }

    pub async fn search(&mut self) -> io::Result<()> {
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
        let repeat = 5;
        let initial_delay = Duration::from_secs(5);
        let resend_every = Duration::from_secs(15 * 60);
        loop {
            let resend_timer = Delay::new(resend_every);
            for _ in 0..repeat {
                let initial_timer = Delay::new(initial_delay);
                outgoing.send((msg.as_bytes(), &MUTLICAST)).await?;
                initial_timer.await;
            }
            resend_timer.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_build() {
        let s = Searcher::custom("splurt", "v0.0.1", "splurt is nice")
            .ip(Ipv4Addr::LOCALHOST)
            .port(1901)
            .mx(3)
            .uuid(Uuid::new_v4())
            .ttl(3)
            .build()
            .expect("built");
        assert_eq!(s.friendly_name, "splurt is nice");
        assert_eq!(s.mx, 3.try_into().expect("mx 3 is valid"));
    }
}
