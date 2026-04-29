#![cfg_attr(unstable_bool_to_result, feature(bool_to_result))]
#![cfg_attr(unstable_never_type, feature(never_type))]

//! Runtime agnostic, non-blocking, non-exclusive async UDP networking.
//!
//! `futures-udp` provides two key structs:
//! - [UdpStream] for reading data from a UDP Socket
//! - [UdpSink] for sending data via a UDP Socket
//!
//! These structs implement the `futures-rs` traits [Stream] & [Sink] respectively but are tested
//! and known to work with both `tokio` & `futures`-rs runtimes. (tokio tests performed in a
//! downstream crate, I'll add them here soon so to make sure this never breaks)
//!
//! ## Why?
//! - I usually don't want to be forced to bring `tokio` into my dependency tree unless I want
//!   to use it as my runtime. I think the runtime choice should be left to the final binary.
//! - `futures-rs` is a lot lighter weight and provided by rust-lang, so I chose that for the base
//!   traits. They are cross-compatible with `tokio`.
//! - Working with a bare UdpSocket is "a bit hard", doing it async is "a bit more hard".
//!   Adding `Stream` & `Sink` semantics makes it "nice".
//! - Despite the docs [futures_net::UdpSocket] creates a blocking socket, which is locked
//!   for exclusive use. (Opening a ticket TBD)
//! 
//! ## Stability & MSRV
//!
//! I've chosen to rely on two experimental features, while this crate is in v0.x.y, as I feel they
//! add significant value to the API. I also believe in supporting language development and
//! generating feedback to features as they near stabilisation.
//! 
//! This crate will not move to v1.x.y until both features are stabilised, or I decide to stop using
//! them. Realistically, however, they will be stable while I allow this API to go through a
//! "settling-in" phase before fixing it at v1.0.0
//!
//! > 🔬 **Experimental Features**
//! >
//! > This crate makes use of the following experimental features:
//! >
//! > - [`#![feature(never_type)]`](https://github.com/rust-lang/rust/issues/35121) [final stages of stabilisation]
//! > - [`#![feature(bool_to_result)]`](https://github.com/rust-lang/rust/issues/142748) [in FCP as of 2026-04-25]
//! >
//! > This list includes any unstable features used by direct & transitive dependencies (currently, none).
//! >
//! > Both are so close to being part of stable rust that I chose to use them here.
//!
//! You do not need to enable these in your own code, the list is for information only.
//!
//! ### Stability guarantees
//!
//! We run automated tests **every month** to ensure no fundamental changes affect this crate and
//! test every PR against the current nightly, as well as the current equivalent beta & stable.
//! If you find an issue before we do, please
//! [raise an issue on github](https://github.com/MusicalNinjaDad/splurt/issues).
//!
//! ### MSRV
//!
//! For those of you working with a pinned nightly (etc.) this crate supports the equivalent of
//! 1.90.0 onwards. We use [autocfg](https://crates.io/crates/autocfg/) to seamlessly handle
//! features which have been stabilised since then.
//!
//! ### Dependencies
//!
//! We deliberately keep the dependency list short and pay attention to any transitive dependencies
//! we bring in.
//! 
//! - `futures-rs` (for the Stream & Sink traits)
//! - `futures-net` (for the underlying UdpSocket)
//! - `socket2` (to set the socket to non-blocking, non-exclusive)
//! ```

use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
    pin::Pin,
    task::{Context, Poll},
};

use futures::{Stream, sink::Sink};
use futures_net::driver::{
    PollEvented,
    sys::{self},
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
    /// Create a new `Self` from a PollEvented<sys::net::UdpSocket>
    fn from_evented_socket(evented_socket: PollEvented<sys::net::UdpSocket>) -> io::Result<Self>;

    /// Create a new `Self` by binding it to a given [SocketAddr].
    ///
    /// In the default implementation, the listener is guaranteed to be constructed to be
    /// non-blocking and have non-exclusive access to the bound address; if either of these system
    /// calls fails to take effect an [io::ErrorKind::Unsupported] will be returned.
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

    /// Get the local address of the underlying Socket
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

    /// Clear the readiness state of the underlying socket.
    ///
    /// **This MUST be called after any failed readiness poll.**
    ///
    /// Implementations should attempt to clear the relevant readiness marker of the underlying
    /// socket and then return:
    /// - `Poll::Pending` if successful
    /// - `Poll::Ready(error)` on error, to avoid repeated polling without handling the error
    ///
    /// #### Note
    /// This returns a `Poll<Result<!>>` which will not currently automatically coerce into a
    /// `Poll<Result<T>>`. Work around this by calling `.map_ok(|x| x)` as a no-op to force the
    /// compiler to notice that everything is fine.
    fn clear_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<!>>;
    // TODO: #30 Should an error during clear_ready be clearly fatal?
    //       One option would be to return a `Poll<Option<!>>`, thus differentiating it
    //       from unblock processing a non-blocking error. This would lead to a Stream
    //       delivering `None` and thus signalling it is dead. But it would lose the
    //       details of the error which occurred.

    /// Checks whether `error` will block the underlying Socket and either:
    /// - calls [Self::clear_ready] for blocking errors
    /// - returns `Poll::Ready(error)` for non-blocking errors
    ///
    /// #### Note
    /// This returns a `Poll<Result<!>>` which will not currently automatically coerce into a
    /// `Poll<Result<T>>`. Work around this by calling `.map_ok(|x| x)` as a no-op to force the
    /// compiler to notice that everything is fine.
    fn unblock(
        self: Pin<&mut Self>,
        error: io::Result<!>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<!>> {
        let Err(error) = error;
        match error.kind() {
            io::ErrorKind::WouldBlock => self.clear_ready(cx),
            _ => Poll::Ready(Err(error)),
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
        let this = self.get_mut();
        let io = &mut this.io;
        Pin::new(&mut *io)
    }

    fn clear_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<!>> {
        match self.as_evented_socket_pin().clear_read_ready(cx) {
            Ok(_) => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        }
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
        let evented_socket = self.as_mut().as_evented_socket_pin();
        match evented_socket.poll_read_ready(cx) {
            Poll::Ready(is_ready) => match is_ready {
                Ok(readiness) => match readiness.is_readable() {
                    true => {
                        let mut buf: [u8; BUF_SIZE] = [b'\x00'; BUF_SIZE];
                        let recv = self
                            .as_socket()
                            .recv_from(&mut buf)
                            .map(|(len, addr)| (buf, len, addr));
                        match recv {
                            Ok(_) => Poll::Ready(Some(recv)),
                            Err(e) => self.unblock(Err(e), cx).map_ok(|x| x).map(Some),
                        }
                    }
                    false => self.clear_ready(cx).map_ok(|x| x).map(Some),
                },
                Err(e) => self.unblock(Err(e), cx).map_ok(|x| x).map(Some),
            },
            Poll::Pending => Poll::Pending,
        }
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
        let this = self.get_mut();
        let io = &mut this.io;
        Pin::new(&mut *io)
    }

    fn clear_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<!>> {
        match self.as_evented_socket_pin().clear_write_ready(cx) {
            Ok(_) => Poll::Pending,
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl<A: ToSocketAddrs> Sink<(&[u8], &A)> for UdpSink {
    type Error = io::Error;

    /// Attempts to prepare the Sink to receive a value.
    ///
    /// This method must be called and return Poll::Ready(Ok(())) prior to each call to start_send.
    ///
    /// This method returns Poll::Ready once the underlying sink is ready to receive data.
    /// If this method returns Poll::Pending, the current task is registered to be notified
    /// (via cx.waker().wake_by_ref()) when poll_ready should be called again.
    ///
    /// If the attempt to poll readiness fails this method will properly handle
    /// it by calling [Self::clear_ready]/[Self::unblock] to ensure the underlying socket does not
    /// remain blocked.
    fn poll_ready(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        let evented_socket = self.as_mut().as_evented_socket_pin();
        match evented_socket.poll_write_ready(cx) {
            Poll::Ready(is_ready) => match is_ready {
                Ok(readiness) => match readiness.is_writable() {
                    true => Poll::Ready(Ok(())),
                    false => self.clear_ready(cx).map_ok(|x| x),
                },
                Err(e) => self.unblock(Err(e), cx).map_ok(|x| x),
            },
            Poll::Pending => Poll::Pending,
        }
    }

    /// #### Note
    /// - While this function will accept multiple addresses, currently data is only sent to the
    ///   first one (TODO)
    /// - If an empty list of addresses the error will be of kind `io::ErrorKind::InvalidInput`
    fn start_send(self: Pin<&mut Self>, item: (&[u8], &A)) -> Result<(), Self::Error> {
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
        <Self as futures::Sink<(&[u8], &A)>>::poll_ready(self, cx)
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
