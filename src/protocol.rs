use derive_more::derive::{Deref, Display, From, Into};
use ipnet::IpNet;
use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Note that his call_id is from internal state and not a SipCallID
#[derive(Debug, From, Into, Deref, Clone, Display, Hash, PartialEq, Eq)]
pub struct InternalCallId(String);

impl InternalCallId {
    pub fn random() -> Self {
        Self(rand::random::<u64>().to_string())
    }
}

#[derive(Debug, Object)]
pub struct SipAuth {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Object)]
pub struct StreamingInfo {
    pub gateway: String,
    pub token: String,
    pub room: String,
    pub peer: String,
}

#[derive(Debug, Enum)]
pub enum OutgoingCallAction {
    End,
}

#[derive(Debug, Deserialize)]
pub struct PhoneNumber {
    pub number: String,
    pub subnets: Vec<IpNet>,
}

#[derive(Debug, Object)]
pub struct CreateCallRequest {
    pub sip_server: String,
    pub sip_auth: Option<SipAuth>,
    pub from_number: String,
    pub to_number: String,
    pub hook: String,
    pub streaming: StreamingInfo,
}

#[derive(Debug, Object)]
pub struct CreateCallResponse {
    pub call_id: String,
    pub ws: String,
}

#[derive(Debug, Object)]
pub struct UpdateCallRequest {
    pub action: OutgoingCallAction,
}

#[derive(Debug, Object)]
pub struct UpdateCallResponse {}

#[derive(Error, Debug)]
pub enum CallApiError {
    #[error("InternalChannel {0}")]
    InternalChannel(String),
    #[error("WrongSecret")]
    WrongSecret,
    #[error("CallNotFound")]
    CallNotFound,
    #[error("SipError {0}")]
    SipError(String),
}

#[derive(Debug, Serialize)]
pub enum OutgoingCallEvent {
    Provisional { code: u16 },
    Early { code: u16 },
    Accepted { code: u16 },
    Bye {},
    Failure { code: u16 },
}
