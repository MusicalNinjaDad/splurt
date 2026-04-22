use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    pin::{Pin, pin},
    task::{Context, Poll, ready},
};

use async_ready::{AsyncReadReady, AsyncWriteReady};
use futures::{AsyncWrite, Stream};
use futures_net::driver::{
    PollEvented,
    sys::{self, event::Ready},
};

#[derive(Debug)]
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

    /// Create a stream to receive messages into `buf`
    pub fn incoming<'buf>(&mut self, buf: &'buf mut [u8]) -> Incoming<'_, 'buf> {
        Incoming {
            inner: self,
            buffer: buf,
        }
    }
}

//Separate struct to allow marking as must_use
#[must_use = "streams do nothing unless polled"]
#[derive(Debug)]
/// Stream returned by [`UdpListener::incoming()`].
///
/// Calling .next().await on a listener creates a UdpStream connected to the source of the
/// next data received
pub struct Incoming<'listener, 'buf> {
    inner: &'listener mut UdpListener,
    buffer: &'buf mut [u8],
}

impl<'listener, 'buf> Stream for Incoming<'listener, 'buf> {
    type Item = io::Result<SocketAddr>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        todo!("poll next")
    }
}

impl AsyncReadReady for UdpListener {
    type Ok = Ready;

    type Err = io::Error;

    fn poll_read_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Self::Ok, Self::Err>> {
        todo!()
    }
}

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
impl AsyncWrite for UdpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        // Cannot use PollEvented directly, as UdpSocket !Write
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

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        todo!()
    }

    fn poll_close(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<io::Result<()>> {
        todo!()
    }
}
