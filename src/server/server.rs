#![deny(warnings)]

use futures_util::stream::SplitSink;
use futures_util::{future, SinkExt, StreamExt, TryStreamExt};
use log::{error, info};
use std::fmt::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use std::{collections::HashMap, env};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::WebSocketStream;

type Tx = Arc<Mutex<SplitSink<WebSocketStream<TcpStream>, Message>>>;
type PeerMap = Arc<Mutex<HashMap<SocketAddr, Tx>>>;

pub async fn run_server() -> Result<(), Error> {
    // This will read the nth argument and unwrap it, if
    // there's no argument provided it will run the
    // anonymous function
    let addr = env::args().nth(2).unwrap_or_else(|| {
        info!("No address provided, using default");
        "127.0.0.1:8080".to_string()
    });

    // I "think" that I could use addr here without & but including it
    // I don't change ownership. This means I can read it later on. If I don't
    // do so, the value is borrowed and not accessible for the info log later.
    let listener = TcpListener::bind(&addr)
        .await
        .expect(&format!("Failed to bind to address {}", addr));

    info!("Listening on port {}", addr);

    // Create a shared state for the connected peers
    let peer_map = Arc::new(Mutex::new(HashMap::<SocketAddr, Tx>::new()));

    // Before this was represented with a while loop on the await and
    // it would extract the result from Ok inline. That meant errors
    // were not getting handled. This seems to be a more robust way to
    // make sure the program doesn't panic upon accepting a TCP connection
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                let peer_map = peer_map.clone();
                tokio::spawn(accept_connection(peer_map, stream));
            }
            Err(e) => {
                error!("Failed to accept connection: {:?}", e);
            }
        }
    }
}

async fn accept_connection(peer_map: PeerMap, stream: TcpStream) {
    let addr = stream.peer_addr().expect("Error retrieving peer address");
    info!("Peer addr: {}", addr);

    // Now that we have established a TCP connection we
    // accept a new web socket connection with it.
    let ws_stream = tokio_tungstenite::accept_async(stream)
        .await
        .expect("Failed to accept a WS connection");
    info!("New socket conn: {}", addr);

    let (write, read) = ws_stream.split();
    let write = Arc::new(Mutex::new(write));

    // Insert the new peer into the shared state
    peer_map.lock().await.insert(addr, write.clone());

    // Clone the peer_map for use in the message loop
    let peer_map_inner = peer_map.clone();

    // It seems like a lot of try_* methods are ones that expect a potential error
    // which I believe is why .expect can be chained.
    // In this case the try_filter attempts to filter incoming messages so that
    // only text and binary are accepted.
    read.try_filter(|msg| future::ready(msg.is_text() || msg.is_binary()))
        .for_each(|msg| {
            let msg = msg.unwrap();
            let peer_map_inner = peer_map_inner.clone();
            let addr = addr.clone();

            async move {
                let peers = peer_map_inner.lock().await;
                for (peer_addr, peer_tx) in peers.iter() {
                    if *peer_addr != addr {
                        let mut peer_tx = peer_tx.lock().await;
                        if let Err(e) = peer_tx.send(msg.clone()).await {
                            error!("Failed to send message to {}: {}", peer_addr, e);
                        }
                    }
                }
            }
        })
        .await;

    // Remove the peer from the shared state when done
    peer_map.lock().await.remove(&addr);
    info!("{} disconnected", addr);
}
