use std::net::SocketAddr;

mod rtp;
mod server;

pub use rtp::{RtpEngineError, RtpEngineOffer};
pub use server::{SipOutgoingCall, SipOutgoingCallError, SipOutgoingCallOut, SipServer, SipServerError};

pub trait IncallValidator: Send + Sync + 'static {
    fn allow(&self, remote: SocketAddr, from: String, to: String) -> Option<()>;
}
