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
/// - [Self::poll_close] will [Self::poll_flush] and then [drop] the [UdpStream] where std-lib
///   implementations of `close` are simple no-ops.
impl AsyncWrite for UdpStream {
    /// Wait for write-readiness then `send` the contents of `buf` to the [Self::connect]ed recipient.
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
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
        ready!(self.as_mut().poll_write_ready(cx)?);
        Poll::Ready(Ok(()))
    }

    /// [Self::poll_flush] and then [drop] the [UdpStream].
    /// 
    /// #### Note
    /// This differs from where `std::net::*` & `futures_net::*` implementations of `close` which
    /// are simple no-ops for TCP and do not exist for UDP.
    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        todo!()
    }
}
