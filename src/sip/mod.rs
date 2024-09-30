use std::net::SocketAddr;

mod rtp;
mod server;

pub use rtp::{create_offer, CreateOfferError, CreatedOffer};
pub use server::{OutgoingCallControl, SipServer, SipServerError};

pub trait IncallValidator: Send + Sync + 'static {
    fn allow(&self, remote: SocketAddr, from: String, to: String) -> Option<()>;
}
