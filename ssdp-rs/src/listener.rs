use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr, SocketAddrV4},
};

use futures::{Stream, StreamExt};
use futures_udp::{EventedUdpSocket, UdpStream};

use crate::{MAX_MSG_SIZE, MULTICAST_IP, SSDP_PORT};

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

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, None)
    }
}
