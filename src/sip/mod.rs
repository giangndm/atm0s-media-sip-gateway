use std::net::SocketAddr;

mod media;
mod server;

pub use media::{MediaApi, MediaEngineError, MediaRtpEngineOffer};
pub use server::{SipOutgoingCall, SipOutgoingCallError, SipOutgoingCallOut, SipServer, SipServerError};

pub trait IncallValidator: Send + Sync + 'static {
    fn allow(&self, remote: SocketAddr, from: String, to: String) -> Option<()>;
}
