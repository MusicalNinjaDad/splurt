use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    pin::Pin,
    task::{Context, Poll, ready},
};

use async_ready::AsyncWriteReady;
use futures::AsyncWrite;
use futures_net::{
    UdpSocket,
    driver::{
        PollEvented,
        sys::{self, event::Ready},
    },
};

pub type UdpListener = UdpSocket;

pub struct UdpStream {
    io: PollEvented<sys::net::UdpSocket>,
    connected_to: Option<SocketAddr>,
}

impl UdpStream {
    pub fn connect(addr: &SocketAddr) -> io::Result<Self> {
        let loopback = Ipv4Addr::new(127, 0, 0, 1);
        let bind_addr: SocketAddr = SocketAddrV4::new(loopback, 0).into();
        let s = sys::net::UdpSocket::bind(&bind_addr)?;
        s.connect(*addr)?;
        let io = PollEvented::new(s);
        Ok(Self {
            io,
            connected_to: Some(*addr),
        })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        let io = &self.io;
        let s = io.get_ref();
        s.local_addr()
    }

    pub fn connected_to(&self) -> Option<SocketAddr> {
        self.connected_to
    }

    pub fn check_connected(&self) -> io::Result<SocketAddr> {
        self.connected_to
            .ok_or(io::Error::from(io::ErrorKind::NotConnected))
    }

    fn clear_write_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> io::Result<()> {
        let socket = Pin::new(&mut self.io);
        socket.clear_write_ready(cx)
    }
}

impl AsyncWriteReady for UdpStream {
    type Ok = Ready;

    type Err = io::Error;

    fn poll_write_ready(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<Self::Ok, Self::Err>> {
        self.io.poll_write_ready(cx)
    }
}

#[expect(unused)]
// Cannot use calls to `PollEvented` directly, as UdpSocket !Write
/// Allows usage of the [std::io::Write] API to [std::net::UdpSocket::send] asynchronously.
/// In particular:
/// - [Self::poll_write], unlike [std::io::Write::write], will automatically queue the
///   current task for wakeup and return if the writer cannot take more data, rather than blocking
///   the calling thread.
/// - [Self::poll_flush], will await write readiness indicating that all pending messages have been
///   sent, then return as a no-op (`UdpSockets` do not have an inherent `flush` method).
/// - [Self::poll_close] remove the internally stored details of the connected [SocketAddr] and
///   connect the underlying system level socket to itself. Using the [UdpStream] again after
///   closing will result in an error. (std-lib implementations of `close` are simple no-ops).
impl AsyncWrite for UdpStream {
    /// Wait for write-readiness then `send` the contents of `buf` to the [Self::connect]ed recipient.
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        self.check_connected()?;
        ready!(self.as_mut().poll_write_ready(cx)?);

        let socket = PollEvented::get_mut(&mut self.io);
        let result = socket.send(buf);
        if let Err(ref e) = result
            && e.kind() == io::ErrorKind::WouldBlock
        {
            self.clear_write_ready(cx)?;
            Poll::Pending
        } else {
            Poll::Ready(result)
        }
    }

    /// Wait for write-readiness to ensure current pending message has been sent.
    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        self.check_connected()?;
        ready!(self.as_mut().poll_write_ready(cx)?);
        Poll::Ready(Ok(()))
    }

    /// Hard closes the [UdpStream], removing the internally stored details of the connected
    /// [SocketAddr] and connecting the underlying system level socket to itself.
    /// Using the [UdpStream] again after closing will result in an error.
    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        let self_socket = self.connected_to().expect("is connected");
        let socket = PollEvented::get_mut(&mut self.io);
        socket.connect(self_socket);
        self.connected_to = None;
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::AsyncWriteExt;
    use futures_net::runtime::Runtime;

    #[cfg(assert_matches_in_root)]
    use std::assert_matches;

    #[cfg(assert_matches_in_module)]
    use std::assert_matches::assert_matches;

    #[futures_net::test]
    async fn not_connected_flush() {
        let loopback = Ipv4Addr::new(127, 0, 0, 1);
        let addr: SocketAddr = SocketAddrV4::new(loopback, 0).into();
        let connected = UdpSocket::bind(&addr).expect("other side");
        let rec_addr = connected.local_addr().expect("bound port");
        let mut sender = UdpStream::connect(&rec_addr).expect("sender");

        sender.close().await.expect("closing sender");
        let flush = sender.flush().await;
        assert_matches!(flush, Err(e) if e.kind() == io::ErrorKind::NotConnected);
    }

    #[futures_net::test]
    async fn not_connected_write() {
        let loopback = Ipv4Addr::new(127, 0, 0, 1);
        let addr: SocketAddr = SocketAddrV4::new(loopback, 0).into();
        let connected = UdpSocket::bind(&addr).expect("other side");
        let rec_addr = connected.local_addr().expect("bound port");
        let mut sender = UdpStream::connect(&rec_addr).expect("sender");

        sender.close().await.expect("closing sender");
        let write_all = sender.write_all(b"foo").await;
        assert_matches!(write_all, Err(e) if e.kind() == io::ErrorKind::NotConnected);
    }
}
