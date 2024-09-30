use std::net::SocketAddr;

mod server;

pub use server::{OutgoingCallControl, SipServer, SipServerError};

pub trait IncallValidator: Send + Sync + 'static {
    fn allow(&self, remote: SocketAddr, from: String, to: String) -> Option<()>;
}
