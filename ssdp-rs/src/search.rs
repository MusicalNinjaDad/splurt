//! An SSDP searcher, that sends M-SEARCH and receives NOTIFY responses
//!
//! ## Example usage
//! ```no_run no reply possible
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
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    task::Poll,
};

use futures::{FutureExt, ready};
use uuid::Uuid;

use crate::{message::Message, udp::UdpStream};

#[derive(Debug)]
pub struct Searcher {
    stream: UdpStream,
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
        Ok(Searcher {
            stream: UdpStream::bind(addr)?,
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
            stream,
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
        Search {
            searcher: stream,
            msg,
        }
    }

    pub fn next<'s>(&'s mut self) -> Next<'s> {
        Next { searcher: self }
    }
}

/// The future returned by [Searcher::next]
pub struct Next<'searcher> {
    searcher: &'searcher mut Searcher,
}

impl<'searcher> Future for Next<'searcher> {
    type Output = Option<io::Result<(String, SocketAddr)>>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = &mut *self;
        let searcher = &mut *this.searcher;
        let stream = &mut searcher.stream;
        let reply = ready!(stream.next().poll_unpin(cx));
        match reply {
            Some(reply) => {
                let (reply, _, sender) = reply?;
                let reply = String::from_utf8_lossy(&reply).to_string();
                Poll::Ready(Some(Ok((reply, sender))))
            }
            None => Poll::Ready(None),
        }
    }
}

/// The future returned by [Searcher::search]
pub struct Search<'searcher> {
    searcher: &'searcher mut UdpStream,
    msg: String,
}

impl<'searcher> Future for Search<'searcher> {
    type Output = io::Result<()>;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let this = &mut *self;
        let stream = &mut *this.searcher;
        let msg = this.msg.as_bytes();
        let ssdp_multicast = Ipv4Addr::new(239, 25, 255, 25);
        let ssdp_multicast = SocketAddr::new(ssdp_multicast.into(), 1900);
        stream
            .push(msg, ssdp_multicast)
            .poll_unpin(cx)
            .map_ok(|_| ())
    }
}
