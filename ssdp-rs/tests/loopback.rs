use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use futures::prelude::*;
use futures_net::{TcpListener, TcpStream, runtime::Runtime};

#[futures_net::test]
async fn tcp() {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    let addr: SocketAddr = SocketAddrV4::new(loopback, 9999).into();
    let mut receiver = TcpListener::bind(&addr).expect("receiver");
    let mut sender = TcpStream::connect(&addr).await.expect("sender");
    let mut received: [u8; 17] = [0; 17];
    let msg: &[u8; 17] = b"tcp loopback test";
    let send = async move {
        println!("sending {}", String::from_utf8_lossy(msg));
        sender.write_all(msg).await.expect("send msg");
        println!("closing sender");
        sender.close().await.expect("closing sender");
    };
    let rec = async {
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
    dbg!(&received);
    assert_eq!(
        String::from_utf8_lossy(&received),
        String::from_utf8_lossy(msg)
    );
}
