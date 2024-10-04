use std::{net::SocketAddr, sync::Arc, time::Duration};

use clap::Parser;
use rust_sip_wp::{
    address_book::{AddressBookStorage, AddressBookSync},
    secure::SecureContext,
    Gateway, GatewayError,
};

/// Sip Gateway for atm0s-media-server
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Address for http server
    #[arg(long, env, default_value = "0.0.0.0:8008")]
    http: SocketAddr,

    /// Address for sip server
    #[arg(long, env, default_value = "0.0.0.0:5060")]
    sip: SocketAddr,

    /// Secret of this gateway
    #[arg(long, env, default_value = "insecure")]
    secret: String,

    /// Address PhoneBook sync for incoming calls
    #[arg(long, env)]
    phone_numbers_sync: Option<String>,

    /// Address PhoneBook sync interval
    #[arg(long, env, default_value_t = 30_000)]
    phone_numbers_sync_interval_ms: u64,

    /// Http hook queues
    #[arg(long, env, default_value_t = 20)]
    http_hook_queues: usize,

    /// MediaServer Gateway
    #[arg(long, env)]
    media_gateway: String,

    /// MediaServer Apps sync endpoint
    #[arg(long, env)]
    media_app_sync: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), GatewayError> {
    tracing_subscriber::fmt::init();
    let args = Args::parse();
    log::info!("Starting server with http port {} and sip port {}", args.http, args.sip);

    let secure_ctx = Arc::new(SecureContext::new(&args.secret));

    let address_book = AddressBookStorage::default();
    if let Some(sync_url) = args.phone_numbers_sync {
        let mut address_book_sync = AddressBookSync::new(&sync_url, Duration::from_millis(args.phone_numbers_sync_interval_ms), address_book.clone());

        tokio::spawn(async move {
            address_book_sync.run_loop().await;
        });
    }

    let mut gateway = Gateway::new(args.http, args.sip, address_book, args.http_hook_queues, &args.media_gateway, secure_ctx).await?;
    loop {
        if let Err(e) = gateway.recv().await {
            log::error!("gateway error {e:?}");
        }
    }
}
