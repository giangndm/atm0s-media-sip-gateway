use std::{io, net::SocketAddr};

use ezk_sip_core::{transport::udp::Udp, Endpoint};
use ezk_sip_ua::{dialog::DialogLayer, invite::InviteLayer};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SipServerError {
    #[error("Unknown error")]
    Unknown,
}

pub struct SipServer {
    endpoint: Endpoint,
}

impl SipServer {
    pub async fn new(addr: SocketAddr) -> io::Result<Self> {
        let mut builder = Endpoint::builder();

        let dialog_layer = builder.add_layer(DialogLayer::default());
        let invite_layer = builder.add_layer(InviteLayer::default());

        Udp::spawn(&mut builder, addr).await?;

        // Build endpoint to start the SIP Stack
        let endpoint = builder.build();

        Ok(Self { endpoint })
    }

    pub async fn recv(&mut self) -> Result<(), SipServerError> {
        todo!()
    }
}
