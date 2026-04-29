//! An SSDP searcher, that sends M-SEARCH and receives NOTIFY responses
//!
//! ## Example usage
//! ```no_run
//! # // no reply possible so not run as test to avoid endless loop
//! # use std::{io, net::Ipv4Addr};
//! use futures::StreamExt;
//! use ssdp_rs::{Listener, Searcher};
//!
//! # fn main() -> io::Result<()> {
//! // Create a new searcher
//! let mut searcher = Searcher::new(
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
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};

use futures::SinkExt;
use futures_timer::Delay;
use futures_udp::{EventedUdpSocket, UdpSink};
use uuid::Uuid;

use crate::{
    MULTICAST, SSDP_PORT,
    message::{Message, Mx},
};

#[derive(Debug)]
pub struct Searcher {
    outgoing: UdpSink,
    /// The MX field in the M-SEARCH message
    pub mx: Mx,
    os: String,
    os_version: String,
    product_name: String,
    product_version: String,
    friendly_name: String,
    uuid: Uuid,
    /// M-SEARCH messages are sent `repeat` times
    repeat: u8,
    /// Delay between repeated M-SEARCH messages
    repeat_delay: Duration,
    /// The full set of `repeat` messages are resent every `resend`
    resend_every: Duration,
}

/// Create a new Searcher with default values as per [UpnpMessenger::build_searcher]
impl Searcher {
    pub fn new(product_name: &str, product_version: &str, friendly_name: &str) -> io::Result<Self> {
        UpnpMessenger::new(product_name, product_version, friendly_name).build_searcher()
    }

    pub fn ttl(&self) -> io::Result<u32> {
        self.outgoing.as_socket().ttl()
    }

    pub fn set_ttl(&mut self, ttl: u32) -> io::Result<u32> {
        let current = self.ttl()?;
        self.outgoing.as_socket_mut().set_ttl(ttl).map(|_| current)
    }

    /// Begin a search loop, this will begin to run endlessly once `await`ed and will send
    ///  `repeat` messages every `repeat_every`. See [UpnpMessenger] for details on how to
    /// customise these values.
    ///
    /// The loop will *not* block in between messages and will only end in case of an error.
    ///
    /// TODO: #35 provide a cancel one-shot channel to search
    pub async fn search(&mut self) -> io::Result<!> {
        let Searcher {
            outgoing,
            mx,
            os,
            os_version,
            product_name,
            product_version,
            friendly_name,
            uuid,
            repeat,
            repeat_delay,
            resend_every,
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
        loop {
            let resend_timer = Delay::new(*resend_every);
            for _ in 0..*repeat {
                let initial_timer = Delay::new(*repeat_delay);
                outgoing.send((msg.as_bytes(), &MULTICAST)).await?;
                initial_timer.await;
            }
            resend_timer.await;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
/// A builder for custom UpnpMessengers
pub struct UpnpMessenger<'bldr> {
    addr: Option<Ipv4Addr>,
    port: Option<u16>,
    ttl: Option<u32>,

    mx: Option<u8>,
    os: Option<&'bldr str>,
    os_version: Option<&'bldr str>,
    product_name: &'bldr str,
    product_version: &'bldr str,
    friendly_name: &'bldr str,
    uuid: Option<Uuid>,
    repeat: Option<u8>,
    repeat_delay: Option<Duration>,
    resend_every: Option<Duration>,
}

impl<'bldr> UpnpMessenger<'bldr> {
    pub fn new(
        product_name: &'bldr str,
        product_version: &'bldr str,
        friendly_name: &'bldr str,
    ) -> UpnpMessenger<'bldr> {
        UpnpMessenger {
            product_name,
            product_version,
            friendly_name,
            ..Default::default()
        }
    }

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

    pub fn os(&mut self, os: &'bldr str) -> &mut Self {
        self.os = Some(os);
        self
    }

    pub fn os_version(&mut self, os_version: &'bldr str) -> &mut Self {
        self.os_version = Some(os_version);
        self
    }

    pub fn uuid(&mut self, uuid: Uuid) -> &mut Self {
        self.uuid = Some(uuid);
        self
    }

    pub fn repeat(&mut self, repeat: u8) -> &mut Self {
        self.repeat = Some(repeat);
        self
    }

    pub fn repeat_delay(&mut self, repeat_delay: Duration) -> &mut Self {
        self.repeat_delay = Some(repeat_delay);
        self
    }

    pub fn resend_every(&mut self, resend_every: Duration) -> &mut Self {
        self.resend_every = Some(resend_every);
        self
    }

    /// Build a Searcher using following default values, if not defined:
    ///
    /// - IP: `Ipv4Addr::UNSPECIFIED`
    /// - Port: 1900
    /// - TTL: 2
    /// - MX: 5 (max)
    /// - OS: retrieved at runtime
    /// - UUID: random UUID v4
    /// - Repeat: 5
    /// - Repeat delay: 5s
    /// - Resend every: 15 mins
    pub fn build_searcher(&mut self) -> io::Result<Searcher> {
        let ip = self.addr.unwrap_or(Ipv4Addr::UNSPECIFIED);
        let port = self.port.unwrap_or(SSDP_PORT);
        let addr = SocketAddrV4::new(ip, port).into();
        let ttl = self.ttl.unwrap_or(2);
        let mut outgoing = UdpSink::bind(addr)?;
        outgoing.as_socket_mut().set_ttl(ttl)?;

        let mx = Mx::try_from(self.mx.unwrap_or(5))?;
        let os_info = osinfo::get();
        let os = self
            .os
            .map(|s| s.to_string())
            .unwrap_or_else(|| os_info.get_name());
        let os_version = self
            .os_version
            .map(|s| s.to_string())
            .unwrap_or_else(|| os_info.get_version().to_string());
        let product_name = self.product_name.to_string();
        let product_version = self.product_version.to_string();
        let friendly_name = self.friendly_name.to_string();
        let uuid = self.uuid.unwrap_or(Uuid::new_v4());
        let repeat = self.repeat.unwrap_or(5);
        let repeat_delay = self.repeat_delay.unwrap_or(Duration::from_secs(5));
        let resend_every = self.resend_every.unwrap_or(Duration::from_secs(15 * 60));
        Ok(Searcher {
            outgoing,
            mx,
            os,
            os_version,
            product_name,
            product_version,
            friendly_name,
            uuid,
            repeat,
            repeat_delay,
            resend_every,
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_build() {
        let os_version = String::from("latest_and_greatest");
        let s = UpnpMessenger::new("splurt", "v0.0.1", "splurt is nice")
            .ip(Ipv4Addr::LOCALHOST)
            .port(1901)
            .mx(3)
            .uuid(Uuid::new_v4())
            .ttl(3)
            .repeat(4)
            .repeat_delay(Duration::from_secs(2))
            .resend_every(Duration::from_secs(60))
            .os("fooOs")
            .os_version(&os_version)
            .build_searcher()
            .expect("built");
        assert_eq!(s.friendly_name, "splurt is nice");
        assert_eq!(s.mx, 3.try_into().expect("mx 3 is valid"));
        assert_eq!(s.repeat, 4);
        assert_eq!(s.repeat_delay, Duration::from_secs(2));
        assert_eq!(s.resend_every, Duration::from_secs(60));
        assert_eq!(s.os, "fooOs");
        assert_eq!(s.os_version, "latest_and_greatest");
    }

    #[test]
    fn new_searcher() {
        let product_name = "splurt";
        let product_version = "v0.0.1";
        let friendly_name = "splurt is nice";
        let s = Searcher::new(product_name, product_version, friendly_name).expect("new searcher");
        let ttl = s.outgoing.as_socket().ttl().expect("socket ttl");
        let bound_addr = s.outgoing.as_socket().local_addr().expect("socket addr");
        let bound_port = bound_addr.port();
        assert_eq!(s.friendly_name, friendly_name);
        assert_eq!(s.product_name, product_name);
        assert_eq!(s.product_version, product_version);
        assert_eq!(s.mx, 5.try_into().expect("default MX 5"));
        assert_eq!(ttl, 2);
        assert_eq!(bound_port, 1900);
    }
}
