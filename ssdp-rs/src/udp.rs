use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    task::{Context, Poll, ready},
};

use async_ready::{AsyncReadReady, AsyncWriteReady};
use futures::{Stream, sink::Sink};
use futures_net::driver::{
    PollEvented,
    sys::{self, event::Ready},
};
use socket2::{Domain, Type};

//TODO: Open a ticket with futures_net re non-blocking UdpSocket
#[derive(Debug)]
/// A non-blocking async UdpSocket with ability to `recv_from` via `next` and `send_to` via `push`.
///
/// #### BUF_SIZE
/// Messages received via [UdpStream::next] will be provided as an array of bytes of length
/// `BUF_SIZE`. This is a generic const to allow avoid us having to allocate a 65k buffer on each
/// call to next in order to cover the max possible UDP datagram size.
///
/// It is your responsibility to ensure that `BUF_SIZE` is large enough to hold the largest UDP
/// datagram your protocol expects; if it is smaller than the incoming datagram size, the datagram
/// will be truncated in the output from `next`. You cannot rely on the returned `bytes_read` value
/// to indicate truncation as this will also be set to the buffer length, not the full size of the
/// truncated message (this is the underlying behaviour of the libc call `recv_from`).
///
/// #### Note
/// - This does NOT have exclusive access to the bound port. If you want to guarantee that
///   no other processes bind to the same socket use a [UdpConnectedStream], which will exclusively
///   claim the port (or vote thumbs up on issue #22 TODO: implement `bind_exclusive` etc.)
pub struct UdpStream<const BUF_SIZE: usize> {
    /// The underlying, evented Socket.
    ///
    /// #### Note
    /// - [`futures_net::UdpSocket`] does NOT implement [futures_net::driver::sys::event::Evented]
    ///   and is NOT the same type as stored here.
    /// - [`futures_net::driver::sys::net::UdpSocket`] is not actually non-blocking, despite the
    ///   documentation.
    /// - Neither [std::sys::net::UdpSocket], nor [net2::UdpBuilder] expose `set_nonblocking()` so
    ///   we need use [socket2::Socket] while building the listener but are unable to change
    ///   blocking or exclusivity after construction.
    io: PollEvented<sys::net::UdpSocket>,
}

/// Basic functions on a struct wrapping a PollEvented<sys::net::UdpSocket>
///
/// Right now this is lazy for my own use, so makes assumptions about internal structure.
///
/// #### Note
/// - TODO #26 handle cases with multiple fields which need to be provided during construction
pub trait EventedUdpSocket
where
    Self: Sized,
{
    /// Create a new thing from a PollEvented<sys::net::UdpSocket>
    fn from_evented_socket(evented_socket: PollEvented<sys::net::UdpSocket>) -> io::Result<Self>;

    /// Create a new [UdpStream] by binding it to a given [SocketAddr].
    ///
    /// The listener is guaranteed to be constructed to be non-blocking and have non-exclusive
    /// access to the bound address; if either of these system calls fails to take effect an
    /// [io::ErrorKind::Unsupported] will be returned.
    fn bind(addr: SocketAddr) -> io::Result<Self> {
        let s2 = socket2::Socket::new(Domain::IPV4, Type::DGRAM, None)?;
        let addr = addr.into();
        s2.set_nonblocking(true)?;
        s2.nonblocking()?
            .ok_or(io::Error::from(io::ErrorKind::Unsupported))?;

        // NOTE for consideration if/when implementing conversion to a UdpConnectedStream
        // ==============================================================================
        // This would stop another process from re-binding to the same address *& port* if
        // converted to a UdpConnectedStream which actively begins listening on this address,
        // thereby claiming exclusive interest in all received data.
        // see https://man7.org/linux/man-pages/man7/socket.7.html#:~:text=SO_REUSEADDR
        s2.set_reuse_address(true)?;
        s2.reuse_address()?
            .ok_or(io::Error::from(io::ErrorKind::Unsupported))?;

        s2.bind(&addr)?;
        let sstd: std::net::UdpSocket = s2.into();
        let evented_socket = PollEvented::new(sys::net::UdpSocket::from_socket(sstd)?);
        Self::from_evented_socket(evented_socket)
    }

    // // TODO: #22 Add bind_exclusive constructor & update struct docs for UdpStream
    // pub fn bind_exclusive(addr: SocketAddr) -> io::Result<Self>
    // pub fn is_exclusive(&self) -> Option<SocketAddr>
    // pub fn check_exclusive(&self) -> io::Result<SocketAddr>
    // pub fn is_non_exclusive(&self) -> Option<SocketAddr>
    // pub fn check_non_exclusive(&self) -> io::Result<SocketAddr>

    /// Get the local address of this listener
    fn local_addr(&self) -> io::Result<SocketAddr> {
        self.as_socket().local_addr()
    }

    /// Provides access to the underlying Socket.
    ///
    /// #### Note
    /// `futures_net::UdpSocket` is NOT the same type as returned here.
    fn as_socket(&self) -> &sys::net::UdpSocket;

    /// Provides mutable access to the underlying Socket.
    ///
    /// #### Note
    /// `futures_net::UdpSocket` is NOT the same type as returned here.
    fn as_socket_mut(&mut self) -> &mut sys::net::UdpSocket;

    /// Converts a pinned `&mut Self` to a pinned &mut of the underlying pollevented socket
    /// allowing for calls to traits and functions implemented by [PollEvented]
    fn as_evented_socket_pin(self: Pin<&mut Self>) -> Pin<&mut PollEvented<sys::net::UdpSocket>>;

    /// Clear the readiness state of this socket, to be called in case of an .
    ///
    /// Implementations should correspond to [poll_ready] and clear the
    /// relevant readiness marker of the underlying socket
    fn clear_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> io::Result<()>;

    /// Checks
    fn would_block(
        self: Pin<&mut Self>,
        error: io::Result<!>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<io::Result<!>>> {
        let Err(error) = error;
        match error.kind() {
            io::ErrorKind::WouldBlock => match self.clear_ready(cx) {
                Ok(_) => Poll::Pending,
                Err(e) => Poll::Ready(Some(Err(e))),
            },
            _ => Poll::Ready(Some(Err(error))),
        }
    }
}

impl<const _BS: usize> EventedUdpSocket for UdpStream<_BS> {
    fn from_evented_socket(evented_socket: PollEvented<sys::net::UdpSocket>) -> io::Result<Self> {
        Ok(Self { io: evented_socket })
    }

    fn as_socket(&self) -> &sys::net::UdpSocket {
        let io = &self.io;
        io.get_ref()
    }

    fn as_socket_mut(&mut self) -> &mut sys::net::UdpSocket {
        let io = &mut self.io;
        io.get_mut()
    }

    fn as_evented_socket_pin(self: Pin<&mut Self>) -> Pin<&mut PollEvented<sys::net::UdpSocket>> {
        let listener = self.get_mut();
        let io = &mut listener.io;
        Pin::new(&mut *io)
    }

    fn clear_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> io::Result<()> {
        self.clear_read_ready(cx)
    }
}

impl<const BUF_SIZE: usize> Stream for UdpStream<BUF_SIZE> {
    type Item = io::Result<([u8; BUF_SIZE], usize, SocketAddr)>;

    /// Receives data from the IO interface once `await`ed.
    ///
    /// Awaiting returns an array of bytes containing the message received, the message length
    /// and the target from whence the data came as an
    /// `Option<io::Result<([u8; BUF_SIZE], usize, SocketAddr)>>`
    ///
    /// #### Note
    ///
    /// - Messages received via [UdpStream::next] will be provided as an array of bytes of length
    ///   `BUF_SIZE`. This is a generic const to allow avoid us having to allocate a 65k buffer on each
    ///   call to next in order to cover the max possible UDP datagram size.
    /// - It is your responsibility to ensure that `BUF_SIZE` is large enough to hold the largest UDP
    ///   datagram your protocol expects; if it is smaller than the incoming datagram size, the datagram
    ///   will be truncated in the output from `next`. You cannot rely on the returned `bytes_read` value
    ///   to indicate truncation as this will also be set to the buffer length, not the full size of the
    ///   truncated message (this is the underlying behaviour of the libc call `recv_from`).
    /// - All bytes after the actual message will be NULL so it can be directly converted to a String,
    ///   for example, without first slicing. Other data manipulation should take into account the actual length.
    /// - There are no clear situations which could lead to this returning `None`. Wrapping the
    ///   returned data in an `Option` is done purely to maintain a consistent API with expectations
    ///   on an Iterator / Stream
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // PollEvented::poll_read_ready consumes the input,unlike PollEvented::poll_write_ready.
        // So we need to reborrow in order to later call clear_read_ready.
        let evented_socket = self.as_mut().as_evented_socket_pin();
        match evented_socket.poll_read_ready(cx) {
            Poll::Ready(result) => match result {
                Ok(ready) => match ready.is_readable() {
                    true => {
                        let mut buf: [u8; BUF_SIZE] = [b'\x00'; BUF_SIZE];
                        let result = self
                            .as_socket()
                            .recv_from(&mut buf)
                            .map(|(len, addr)| (buf, len, addr));
                        match result {
                            Err(error) => self.would_block(Err(error), cx).map_ok(|x| x),
                            _ => Poll::Ready(Some(result)),
                        }
                    }
                    false => {
                        let evented_socket = self.as_mut().as_evented_socket_pin();
                        evented_socket.clear_read_ready(cx)?;
                        Poll::Pending
                    }
                },
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => {
                        let evented_socket = self.as_mut().as_evented_socket_pin();
                        evented_socket.clear_read_ready(cx)?;
                        Poll::Pending
                    }
                    _ => Poll::Ready(Some(Err(e))),
                },
            },
            Poll::Pending => Poll::Pending,
        }
    }
}

mod useful_docs {
    use super::*;

    impl<const BUF_SIZE: usize> UdpStream<BUF_SIZE> {
        /// Receives data from the IO interface once `await`ed.
        ///
        /// Awaiting returns the number of bytes read and the target from whence the data as an
        /// `io::Result<(usize, SocketAddr)>`
        pub fn recv_from<'listener, 'buf>(
            &'listener mut self,
            buf: &'buf mut [u8],
        ) -> RecvFrom<'listener, 'buf, BUF_SIZE> {
            RecvFrom {
                buf,
                listener: self,
            }
        }
    }
}

/// The future returned by `UdpStream::recv_from`
#[derive(Debug)]
pub struct RecvFrom<'listener, 'buf, const _BUF_SIZE: usize> {
    listener: &'listener mut UdpStream<_BUF_SIZE>,
    buf: &'buf mut [u8],
}

impl<'listener, 'buf, const _BS: usize> Future for RecvFrom<'listener, 'buf, _BS> {
    type Output = io::Result<(usize, SocketAddr)>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let RecvFrom { listener, buf } = &mut *self;
        Pin::new(&mut **listener).poll_recv_from(cx, buf)
    }
}

/// Private functions for handling readiness to read.
impl<const BUF_SIZE: usize> UdpStream<BUF_SIZE> {
    /// Receives data from the IO interface if it is ready to read.
    ///
    /// If successful, returns the number of bytes read and the target from whence the data came.
    fn poll_recv_from(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<Result<(usize, std::net::SocketAddr), io::Error>> {
        let listener = &mut *self;
        ready!(listener.poll_read_ready_unpin(cx)?);

        let socket = listener.as_socket();
        let result = socket.recv_from(buf);

        if let Err(ref e) = result
            && e.kind() == io::ErrorKind::WouldBlock
        {
            self.clear_read_ready(cx)?;
            Poll::Pending
        } else {
            Poll::Ready(result)
        }
    }

    /// Converts a pinned `&mut UdpStream` to a pinned &mut of the underlying pollevented socket
    /// allowing for calls to traits and functions implemented by [PollEvented]
    fn pinned_io(self: Pin<&mut Self>) -> Pin<&mut PollEvented<sys::net::UdpSocket>> {
        let listener = self.get_mut();
        let io = &mut listener.io;
        Pin::new(&mut *io)
    }

    /// Needed to handle non-blocking errors in [futures::AsyncRead].
    /// See [futures_net::driver::PollEvented] for an explanation.
    fn clear_read_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> io::Result<()> {
        self.pinned_io().clear_read_ready(cx)
    }
}

impl<const _BUF_SIZE: usize> AsyncReadReady for UdpStream<_BUF_SIZE> {
    type Ok = Ready;

    type Err = io::Error;

    fn poll_read_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Self::Ok, Self::Err>> {
        self.pinned_io().poll_read_ready(cx)
    }
}

impl<const _BUF_SIZE: usize> AsyncWriteReady for UdpStream<_BUF_SIZE> {
    type Ok = Ready;

    type Err = io::Error;

    fn poll_write_ready(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Self::Ok, Self::Err>> {
        self.pinned_io().poll_write_ready(cx)
    }
}

#[derive(Debug)]
/// A non-blocking async UdpSocket with ability to `send_to` via `send` and make use of all the
/// niceties that come with [`futures::sink::Sink`] and [`futures::sink::SinkExt`].
///
/// #### Note
/// - This does NOT have exclusive access to the bound port. If you want to guarantee that
///   no other processes bind to the same socket use a [UdpConnectedStream], which will exclusively
///   claim the port (or vote thumbs up on issue #22 TODO: implement `bind_exclusive` etc.)
pub struct UdpSink {
    /// The underlying, evented Socket.
    ///
    /// #### Note
    /// - [`futures_net::UdpSocket`] does NOT implement [futures_net::driver::sys::event::Evented]
    ///   and is NOT the same type as stored here.
    /// - [`futures_net::driver::sys::net::UdpSocket`] is not actually non-blocking, despite the
    ///   documentation.
    /// - Neither [std::sys::net::UdpSocket], nor [net2::UdpBuilder] expose `set_nonblocking()` so
    ///   we need use [socket2::Socket] while building the listener but are unable to change
    ///   blocking or exclusivity after construction.
    io: PollEvented<sys::net::UdpSocket>,
}

impl EventedUdpSocket for UdpSink {
    fn from_evented_socket(evented_socket: PollEvented<sys::net::UdpSocket>) -> io::Result<Self> {
        Ok(Self { io: evented_socket })
    }

    fn as_socket(&self) -> &sys::net::UdpSocket {
        let io = &self.io;
        io.get_ref()
    }

    fn as_socket_mut(&mut self) -> &mut sys::net::UdpSocket {
        let io = &mut self.io;
        io.get_mut()
    }

    fn as_evented_socket_pin(self: Pin<&mut Self>) -> Pin<&mut PollEvented<sys::net::UdpSocket>> {
        let listener = self.get_mut();
        let io = &mut listener.io;
        Pin::new(&mut *io)
    }

    fn clear_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> io::Result<()> {
        todo!("sink clear_ready")
    }
}

impl<A: ToSocketAddrs> Sink<(&[u8], &A)> for UdpSink {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let socket = self.as_evented_socket_pin();
        // Not sure whether PollEvented::poll_write_ready() returning a `Result<Ready>`
        // conveys additional meaningful information, so for saftey not simply using
        // `ready!(...map(|_| ()))` and instead double-checking readiness kind
        match socket.poll_write_ready(cx) {
            Poll::Ready(result) => match result {
                Ok(ready) => match ready.is_writable() {
                    true => Poll::Ready(Ok(())),
                    false => {
                        //TODO could this be nastily fatal?
                        //     it's what futures_net does in `impl AsyncRead/Write for PollEvented`
                        socket.clear_write_ready(cx)?;
                        Poll::Pending
                    }
                },
                Err(e) => match e.kind() {
                    io::ErrorKind::WouldBlock => {
                        //TODO could this be nastily fatal?
                        //     it's what futures_net does in `impl AsyncRead/Write for PollEvented`
                        socket.clear_write_ready(cx)?;
                        Poll::Pending
                    }
                    _ => Poll::Ready(Err(e)),
                },
            },
            Poll::Pending => Poll::Pending,
        }
    }

    /// #### Note
    /// - While this function will accept multiple addresses, currently data is only sent to the
    ///   first one (TODO)
    /// - If an empty list of addresses the error will be of kind `io::ErrorKind::InvalidInput`
    fn start_send(self: Pin<&mut Self>, item: (&[u8], &A)) -> Result<(), Self::Error> {
        //TODO Implementations of poll_ready and start_send will usually involve flushing behind
        //     the scenes in order to make room for new messages.

        let socket = self.as_socket();
        let (msg, addr) = item;
        let addr = addr
            .to_socket_addrs()?
            .next()
            .ok_or(io::Error::from(io::ErrorKind::InvalidInput))?;
        socket.send_to(msg, &addr).and_then(|l| {
            if l != msg.len() {
                Err(io::Error::other(format!(
                    "{} bytes sent but message was {} bytes",
                    l,
                    msg.len()
                )))
            } else {
                Ok(())
            }
        })
    }

    /// Await write readiness indicating that all pending messages have been sent, then return
    /// as a no-op (`UdpSockets` do not have an inherent `flush` method).
    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_evented_socket_pin().poll_write_ready(cx)?);
        Poll::Ready(Ok(()))
    }

    // TODO: it would be nice to be able to annote these situations with as eg `Poll<!>`
    //       is this worth a sub-issue to the tracking issue for `never_type`?
    /// #### Note
    /// This only flushes but does not close as no-one exposes the libc `close()`
    /// call on a `UdpSocket`
    fn poll_close(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        <Self as futures::Sink<(&[u8], &A)>>::poll_flush(self, cx)
    }
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, SocketAddrV4};

    use super::*;
    use futures::{SinkExt, StreamExt};
    use futures_net::runtime::Runtime;

    #[futures_net::test]
    async fn non_blocking() {
        let loopback = Ipv4Addr::new(127, 0, 0, 1);
        let addr: SocketAddr = SocketAddrV4::new(loopback, 0).into();
        let first = UdpStream::<32>::bind(addr).expect("first connection");
        let addr = first.local_addr().expect("bound port");
        let _second = UdpStream::<32>::bind(addr).expect("second connection");
    }

    #[futures_net::test]
    async fn truncated_next() {
        let loopback = Ipv4Addr::new(127, 0, 0, 1);
        let addr: SocketAddr = SocketAddrV4::new(loopback, 0).into();
        let mut receiver = UdpStream::<8>::bind(addr).expect("receiver");
        let rec_addr = receiver.local_addr().expect("bound port");

        let mut sender = UdpSink::bind(addr).expect("sender");
        let original_msg = b"udp loopback test";

        let send = async move {
            sender
                .send((original_msg, &rec_addr))
                .await
                .expect("send msg");
        };

        let rec = async {
            let (msg, len, _sent_by) = receiver
                .next()
                .await
                .expect("a message")
                .expect("a valid message");
            // bytes read is limited to buf size - as per libc call recv_from
            assert_eq!(len, 8);
            assert_eq!(msg, original_msg[..8]);
        };

        futures::join!(rec, send);
    }
}
