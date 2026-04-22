use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use futures::prelude::*;
use futures_net::{TcpListener, TcpStream, runtime::Runtime};

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
async fn udp() {
    let loopback = Ipv4Addr::new(127, 0, 0, 1);
    
    // https://doc.rust-lang.org/std/net/struct.TcpListener.html#method.bind
    // Binding with a port number of 0 will request that the OS assigns a port to this listener.
    // The port allocated can be queried via the TcpListener::local_addr method.
    let addr: SocketAddr = SocketAddrV4::new(loopback, 0).into();
    #[expect(unused_mut)]
    let mut receiver = TcpListener::bind(&addr).expect("receiver");
    let addr = receiver.local_addr().expect("bound port");
    dbg!(addr);

    // let mut sender = TcpStream::connect(&addr).await.expect("sender");
    
    // let mut received: [u8; 17] = [b'\x00'; 17];
    // let msg: &[u8; 17] = b"udp loopback test";
    
    // let send = async move {
    //     println!("sending {}", String::from_utf8_lossy(msg));
    //     sender.write_all(msg).await.expect("send msg");
    //     println!("closing sender");
    //     sender.close().await.expect("closing sender");
    // };
    
    // let rec = async {
    //     println!("initiating receiver");
    //     if let Some(stream) = receiver.incoming().next().await {
    //         println!("receiving ...");
    //         stream
    //             .expect("valid stream")
    //             .read_exact(&mut received)
    //             .await
    //             .expect("received something");
    //         println!("received: {}", String::from_utf8_lossy(&received));
    //     };
    // };

    // println!("ready to join");
    // futures::join!(rec, send);
    
    // assert_eq!(
    //     String::from_utf8_lossy(&received),
    //     String::from_utf8_lossy(msg)
    // );
}
