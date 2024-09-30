use poem_openapi::{Enum, Object};

#[derive(Debug, Object)]
pub struct SipAuth {
    username: String,
    password: String,
}

#[derive(Debug, Object)]
pub struct StreamingInfo {
    gateway: String,
    token: String,
    room: String,
    peer: String,
}

#[derive(Debug, Enum)]
pub enum OutgoingCallAction {
    End,
}
