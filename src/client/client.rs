use futures_util::{future, pin_mut, StreamExt};
use log::info;
use std::env;
use std::fmt::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

pub async fn run_client() -> Result<(), Error> {
    // This will read the nth argument and unwrap it, if
    // there's no argument provided it will run the
    // anonymous function
    let addr = env::args().nth(2).unwrap_or_else(|| {
        info!("No address provided, using default");
        "ws://127.0.0.1:8080/".to_string()
    });

    // The thing I find confusing and I"m not sure how idiomatic it is,
    // is that you pretty much import inline and use right away. So there's
    // no use futures_channel and so on, I just do it all exactly where I need
    // to.
    let (stdin_tx, stdin_rx) = futures_channel::mpsc::unbounded();
    // This seems to make sense to me, we have a new thread or whatever Rust
    // uses to continuously read from std input.
    tokio::spawn(read_stdin(stdin_tx));

    let (ws_stream, _) = connect_async(&addr)
        .await
        .expect(&format!("Failed to connect to {}", addr));
    println!("WebSocket handshake has been completed");

    let (write, read) = ws_stream.split();
    let stdin_to_ws = stdin_rx.map(Ok).forward(write);
    let ws_to_stdout = {
        read.for_each(|message| async {
            let data = message.unwrap().into_data();
            let mut prefixed_data = b"> ".to_vec();
            prefixed_data.extend_from_slice(&data);
            tokio::io::stdout().write_all(&prefixed_data).await.unwrap();
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;
    Ok(())
}

async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
    let mut stdin = tokio::io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx.unbounded_send(Message::binary(buf)).unwrap();
    }
}
