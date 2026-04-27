use std::{
    net::{Ipv4Addr, SocketAddr, SocketAddrV4},
    str::FromStr,
};

use futures::prelude::*;
use futures_net::{TcpListener, TcpStream, runtime::Runtime};
use ssdp_rs::udp::{EventedUdpSocket, UdpSink, UdpStream};

#[futures_net::test]
async fn tcp() {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);

    // https://doc.rust-lang.org/std/net/struct.TcpListener.html#method.bind
    // Binding with a port number of 0 will request that the OS assigns a port to this listener.
    // The port allocated can be queried via the TcpListener::local_addr method.
    let addr: SocketAddr = SocketAddrV4::new(loopback, 0).into();
    let mut receiver = TcpListener::bind(&addr).expect("receiver");
    let addr = receiver.local_addr().expect("bound port");
    dbg!(addr);

    let mut sender = TcpStream::connect(&addr).await.expect("sender");

    let mut received: [u8; 17] = [b'\x00'; 17];
    let msg: &[u8; 17] = b"tcp loopback test";

    let send = async move {
        println!("sending {}", String::from_utf8_lossy(msg));
        sender.write_all(msg).await.expect("send msg");
        println!("closing sender");
        sender.close().await.expect("closing sender");
    };

    let rec = async {
        println!("initiating receiver");
        if let Some(stream) = receiver.incoming().next().await {
            println!("receiving ...");
            stream
                .expect("valid stream")
                .read_exact(&mut received)
                .await
                .expect("received something");
            println!("received: {}", String::from_utf8_lossy(&received));
        };
    };

    println!("ready to join");
    futures::join!(rec, send);

    assert_eq!(
        String::from_utf8_lossy(&received),
        String::from_utf8_lossy(msg)
    );
}

#[futures_net::test]
async fn stream_and_sink() {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);

    // https://doc.rust-lang.org/std/net/struct.TcpListener.html#method.bind
    // Binding with a port number of 0 will request that the OS assigns a port to this listener.
    // The port allocated can be queried via the TcpListener::local_addr method.
    let addr: SocketAddr = SocketAddrV4::new(loopback, 0).into();

    let mut sender = UdpSink::bind(addr).expect("sender");
    let send_addr = sender.local_addr().expect("bound port");
    let msg: &[u8; 17] = b"udp loopback test";
    dbg!(send_addr);

    let mut receiver = UdpStream::<32>::bind(addr).expect("receiver");
    let rec_addr = receiver.local_addr().expect("bound port");
    let mut received: [u8; 17] = [b'\x00'; 17];
    // dummy address (google DNS) should be changed on reception of message from our sender
    let mut outer_sent_by = SocketAddr::from_str("8.8.8.8:80").expect("valid addr");
    dbg!(rec_addr);

    let send = async move {
        println!("sending {}", String::from_utf8_lossy(msg));
        sender.send((msg, &rec_addr)).await.expect("send msg");
    };

    let rec = async {
        println!("initiating receiver");
        let (msg, len, sent_by) = receiver
            ._next()
            .await
            .expect("a message")
            .expect("a valid message");
        println!(
            "received: {} from {} ({} bytes)",
            String::from_utf8_lossy(&msg),
            sent_by,
            len
        );
        received = msg[..len].try_into().expect("17 bytes in msg");
        outer_sent_by = sent_by;
    };

    println!("ready to join");
    futures::join!(rec, send);

    assert_eq!(
        String::from_utf8_lossy(&received),
        String::from_utf8_lossy(msg)
    );

    assert_eq!(outer_sent_by, send_addr)
}
