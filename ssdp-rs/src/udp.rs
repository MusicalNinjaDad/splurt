use std::{
    io,
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    pin::Pin,
    task::{Context, Poll, ready},
};

use async_datagram::AsyncDatagram;
use async_ready::AsyncWriteReady;
use futures::{AsyncWrite, AsyncWriteExt};
use futures_net::driver::{
    PollEvented,
    sys::{self, event::Ready},
};
use socket2::{Domain, Type};

//TODO: Open a ticket with futures_net re non-blocking UdpSocket
#[derive(Debug)]
/// A non-blocking async UdpSocket semantically designed as a listener.
///
/// #### Note
/// This does NOT have exclusive access to the bound port. If you want to guarantee that no other
/// processes bind to the same socket use a [UdpStream], which will exclusively claim the port.
pub struct UdpListener {
    /// The underlying, evented Socket.
    ///
    /// #### Note
    /// - [`futures_net::UdpSocket`] does NOT implement [futures_net::driver::sys::event::Evented]
    ///   and is NOT the same type as stored here.
    /// - [`futures_net::driver::sys::net::UdpSocket`] is not actually non-blocking, despite the
    ///   documentation.
    /// - Neither [std::sys::net::UdpSocket], nor [net2::UdpBuilder] expose`set_nonblocking()` so
    ///   we need use [socket2::Socket] while building the listener.
    io: PollEvented<sys::net::UdpSocket>,
}

impl UdpListener {
    pub fn bind(addr: SocketAddr) -> io::Result<Self> {
        let s2 = socket2::Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        let addr = addr.into();
        s2.set_nonblocking(true)?;
        s2.nonblocking()?
            .ok_or(io::Error::from(io::ErrorKind::Unsupported))?;

        // NOTE for consideration if/when implementing conversion to a UdpStream
        // =====================================================================
        // This would still another process from re-binding to the same address & port if converted
        // to a UdpStream which actively begins listening on this address, thereby claiming
        // exclusive interest in all received data.
        // see https://man7.org/linux/man-pages/man7/socket.7.html#:~:text=SO_REUSEADDR
        s2.set_reuse_address(true)?;
        s2.reuse_address()?
            .ok_or(io::Error::from(io::ErrorKind::Unsupported))?;

        s2.bind(&addr)?;
        let sstd: std::net::UdpSocket = s2.into();
        let io = PollEvented::new(sys::net::UdpSocket::from_socket(sstd)?);
        Ok(Self { io })
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        let io = &self.io;
        let s = io.get_ref();
        s.local_addr()
    }

    pub fn recv_from<'listener, 'buf>(
        &'listener mut self,
        buf: &'buf mut [u8],
    ) -> RecvFrom<'listener, 'buf> {
        RecvFrom {
            buf,
            listener: self,
        }
    }
}

/// The future returned by `UdpListener::recv_from`
#[derive(Debug)]
pub struct RecvFrom<'listener, 'buf> {
    listener: &'listener mut UdpListener,
    buf: &'buf mut [u8],
}

impl<'listener, 'buf> Future for RecvFrom<'listener, 'buf> {
    type Output = io::Result<(usize, SocketAddr)>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let RecvFrom { listener, buf } = &mut *self;
        Pin::new(&mut **listener).poll_recv_from(cx, buf)
    }
}

impl AsyncDatagram for UdpListener {
    type Sender = std::net::SocketAddr;

    type Receiver = std::net::SocketAddr;

    type Err = io::Error;

    #[expect(unused_variables)]
    fn poll_send_to(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
        receiver: &Self::Receiver,
    ) -> Poll<Result<usize, Self::Err>> {
        todo!("poll send to")
    }

    fn poll_recv_from(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        _buf: &mut [u8],
    ) -> Poll<Result<(usize, Self::Sender), Self::Err>> {
        todo!("poll recv from")
    }
}

pub struct UdpStream {
    /// The underlying, evented Socket.
    ///
    /// Note: `futures_net::UdpSocket` does NOT implement [futures_net::driver::sys::event::Evented]
    /// and is NOT the same type as stored here.
    io: PollEvented<sys::net::UdpSocket>,
    connected_to: Option<SocketAddr>,
}

impl UdpStream {
    /// Create a new UpdStream on an OS-assigned port on `loopback` which is connected to `addr`.
    ///
    /// All `write`s will send to `addr` and only packets from `addr` will be provided by `read`.
    pub fn new(addr: &SocketAddr) -> io::Result<Self> {
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

    /// Connect an existing UpdStream to a new counterpart address.
    ///
    /// All `write`s will send to `addr` and only packets from `addr` will be provided by `read`.
    pub fn connect(&mut self, addr: &SocketAddr) -> io::Result<()> {
        let socket = self.io.get_mut();
        socket.connect(*addr)?;
        self.connected_to = Some(*addr);
        Ok(())
    }

    /// Get the local address of this Stream
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        let io = &self.io;
        let s = io.get_ref();
        s.local_addr()
    }

    /// Get the IP-Address of the couterpart to which the Stream is connected.
    ///
    /// All `writes` will be sent to this address and `read`s will be filtered to only provide
    /// packets from this address.
    ///
    /// If the `UdpStream` has been disconnected via `.close()`, this will return `None`.
    ///
    /// Prefer `.check_connected()?` if you need to validate that socket is connected before
    /// continuing.
    pub fn connected_to(&self) -> Option<SocketAddr> {
        self.connected_to
    }

    /// Returns an [io::Error] with [io::ErrorKind::NotConnected] if the connection has been
    /// disconnected via `.close()`.
    ///
    /// Prefer `.connected_to()` if you simply wish to get the address of the connection.
    ///
    /// #### Note:
    /// It is *undefined behaviour to attempt to use a UdpStream which is not connected to a
    /// counterpart. All provided methods use `check_connected` where needed. See [Self::poll_write]
    /// for an example if you are implementing your own function.
    pub fn check_connected(&self) -> io::Result<SocketAddr> {
        self.connected_to
            .ok_or(io::Error::from(io::ErrorKind::NotConnected))
    }

    /// Provides access to the underlying Socket.
    ///
    /// #### Note
    /// `futures_net::UdpSocket` does NOT implement [futures_net::driver::sys::event::Evented]
    /// and is NOT the same type as returned here.
    pub fn socket(&mut self) -> &mut sys::net::UdpSocket {
        PollEvented::get_mut(&mut self.io)
    }

    /// Needed to handle non-blocking errors in [futures::AsyncWrite].
    /// See [futures_net::driver::PollEvented] for an explanation.
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

        let socket = self.socket();
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

    /// SOFT-closes the [UdpStream], removing the internally stored details of the connected
    /// [SocketAddr] and connecting the underlying system level socket to itself.
    /// Using the [UdpStream] while closed will result in an error.
    ///
    /// #### Note:
    /// - Call [Self::flush] first if you want to ensure that any waiting packets are sent
    /// - This will NOT release the underlying [UdpSocket] backing the [UdpStream] for the OS to
    ///   reuse. The Stream can be re-connected to a new partner with [Self::connect]
    fn poll_close(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::result::Result<(), std::io::Error>> {
        let self_socket = self.local_addr()?;
        let socket = self.socket();
        socket.connect(self_socket)?;
        self.connected_to = None;
        Poll::Ready(Ok(()))
    }
}

/// Shadow functions provided to override default documentation
impl UdpStream {
    /// Wait for write-readiness to ensure current pending message has been sent.
    pub fn flush(&mut self) -> futures::io::Flush<'_, Self> {
        <Self as AsyncWriteExt>::flush(self)
    }

    /// "Closes" the [UdpStream], removing the internally stored details of the connected
    /// [SocketAddr] and connecting the underlying system level socket to itself.
    /// Using the [UdpStream] while closed will result in a runtime error.
    ///
    /// #### Note:
    /// This will NOT release the underlying [UdpSocket] backing the [UdpStream] for the OS to
    /// reuse. That only occurs on `drop`.
    /// The Stream can be re-connected to a new counterpart with [Self::connect]
    pub fn close(&mut self) -> futures::io::Close<'_, Self> {
        <Self as AsyncWriteExt>::close(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::AsyncWriteExt;
    use futures_net::{UdpSocket, runtime::Runtime};

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
        let mut sender = UdpStream::new(&rec_addr).expect("sender");

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
        let mut sender = UdpStream::new(&rec_addr).expect("sender");

        sender.close().await.expect("closing sender");
        let write_all = sender.write_all(b"foo").await;
        assert_matches!(write_all, Err(e) if e.kind() == io::ErrorKind::NotConnected);
    }

    #[futures_net::test]
    async fn reuse_socket() {
        let loopback = Ipv4Addr::new(127, 0, 0, 1);
        let addr: SocketAddr = SocketAddrV4::new(loopback, 0).into();
        let connected = UdpSocket::bind(&addr).expect("other side");
        let rec_addr = connected.local_addr().expect("bound port");
        let mut sender = UdpStream::new(&rec_addr).expect("sender");
        assert_matches!(sender.connected_to(), Some(addr) if addr == rec_addr);

        sender.close().await.expect("closing sender");
        let connected = UdpSocket::bind(&addr).expect("other side");
        let new_rec_addr = connected.local_addr().expect("bound port");
        assert_ne!(new_rec_addr, rec_addr);
        let _ = sender.connect(&new_rec_addr);
        assert_matches!(sender.connected_to(), Some(a) if a == new_rec_addr, "should be connected to {new_rec_addr}");
        sender
            .write_all(b"foo")
            .await
            .expect("can write after reconnect");
    }

    #[futures_net::test]
    async fn double_close() {
        let loopback = Ipv4Addr::new(127, 0, 0, 1);
        let addr: SocketAddr = SocketAddrV4::new(loopback, 0).into();
        let connected = UdpSocket::bind(&addr).expect("other side");
        let rec_addr = connected.local_addr().expect("bound port");
        let mut sender = UdpStream::new(&rec_addr).expect("sender");
        sender.close().await.expect("close 1");
        sender.close().await.expect("close 2");
        assert!(!sender.connected_to().is_some());
    }

    #[futures_net::test]
    async fn non_blocking() {
        let loopback = Ipv4Addr::new(127, 0, 0, 1);
        let addr: SocketAddr = SocketAddrV4::new(loopback, 0).into();
        let first = UdpListener::bind(addr).expect("first connection");
        let addr = first.local_addr().expect("bound port");
        let _second = UdpListener::bind(addr).expect("second connection");
    }
}
