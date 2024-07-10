#![deny(warnings)]

mod client;
mod server;

use log::{error, info};
use std::{env, fmt::Error};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initializing a global logger, this allows me to use
    // macros like info!, warn!, and error!
    // I have to run RUST_LOG=info cargo run for the specific
    // log to show (info in this case)
    let _ = env_logger::try_init().expect("Error initializing global logger");

    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        error!("Usage: {} <server|client>", args[0]);
        std::process::exit(1);
    }

    match args[1].as_str() {
        "server" => {
            info!("We're alive");
            server::server::run_server().await
        }
        "client" => client::client::run_client().await,
        _ => {
            error!("Unknown argument; {}", args[1]);
            std::process::exit(1);
        }
    }
}
