use std::{io, net::SocketAddr};

use http::HttpServer;
use thiserror::Error;

pub mod http;
pub mod protocol;
pub mod sip;

#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("IoError {0}")]
    Io(#[from] io::Error),
}

pub struct Gateway {
    http: SocketAddr,
}

impl Gateway {
    pub async fn new(http: SocketAddr, sip: SocketAddr) -> Result<Self, GatewayError> {
        Ok(Self { http })
    }

    pub async fn run_loop(&mut self) -> Result<(), GatewayError> {
        let mut http = HttpServer::new(self.http);
        http.run_loop().await?;
        Ok(())
    }
}
