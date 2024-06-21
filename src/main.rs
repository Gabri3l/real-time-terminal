#![deny(warnings)]

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
};

const DEFAULT_ADDRESS_AND_PORT: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // When I use the await it means I want to wait this asynchronous request
    // to complete before moving forward. The ? indicates that it can fail and
    // it will return the error to the caller, so it bubbles that up.
    // This is possible if the function this is in has a return value matching
    // the error returned potentially by this async call.
    let listener = TcpListener::bind(DEFAULT_ADDRESS_AND_PORT).await?;

    loop {
        // Here I have a tuple returned from listener.accept, the first value
        // is declared as mutable. While I wasn't sure why this is necessary here
        // it seems that the next calls to read/write_all expect the socket to be
        // mutable (TODO: will have to dig more on this)
        let (mut socket, _) = listener.accept().await?;

        tokio::spawn(async move {
            let mut buf = [0 as u8; 1024];

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("failed to read from socket; err ={:?}", e);
                        return;
                    }
                };

                if let Err(e) = socket.write_all(&buf[0..n]).await {
                    eprintln!("failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}
