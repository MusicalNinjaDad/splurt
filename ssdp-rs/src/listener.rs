use std::{io, net::SocketAddr};

use futures_net::driver::{PollEvented, sys};

pub struct UdpListener {
    io: PollEvented<sys::net::UdpSocket>,
}

impl UdpListener {
    /// Creates a UDP socket from the given address.
    ///
    /// Binding with a port number of 0 will request that the OS assigns a port to this listener.
    /// The port allocated can be queried via the UdpListener::local_addr method.
    pub fn bind(addr: &SocketAddr) -> io::Result<UdpListener> {
        let s = sys::net::UdpSocket::bind(addr)?;
        let io = PollEvented::new(s);
        Ok(UdpListener { io })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        let io = &self.io;
        let s = io.get_ref();
        s.local_addr()
    }
}
#[cfg(test)]
mod tests {}
