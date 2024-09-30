use std::net::SocketAddr;

use clap::Parser;
use rust_sip_wp::{Gateway, GatewayError};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Address for http server
    #[arg(long, default_value = "0.0.0.0:8008")]
    http: SocketAddr,

    /// Address for sip server
    #[arg(long, default_value = "0.0.0.0:5060")]
    sip: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), GatewayError> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    log::info!(
        "Starting server with http port {} and sip port {}",
        args.http,
        args.sip
    );

    let mut gateway = Gateway::new(args.http, args.sip).await?;
    gateway.run_loop().await
}
