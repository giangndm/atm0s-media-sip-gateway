use std::{collections::HashMap, net::SocketAddr};

use derive_more::derive::{Display, From};
use outgoing_call::OutgoingCall;
use serde::Serialize;
use thiserror::Error;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use crate::{
    address_book::AddressBookStorage,
    protocol::{CallApiError, CreateCallRequest, CreateCallResponse, InternalCallId, UpdateCallRequest, UpdateCallResponse},
    sip::SipServer,
};

pub mod incoming_call;
pub mod outgoing_call;

#[derive(From, PartialEq, Eq, Hash, Clone, Copy, Display)]
pub struct EmitterId(u64);

impl EmitterId {
    pub fn rand() -> Self {
        Self(rand::random())
    }
}

#[derive(Debug, Error)]
pub enum EventEmitterError {
    #[error("serialize error")]
    SerializeError,
    #[error("internal channel error")]
    InternalChannel,
}

pub trait EventEmitter: Send + Sync + 'static {
    fn emitter_id(&self) -> EmitterId;
    fn fire<E: Serialize>(&mut self, event: &E);
}

pub struct CallManager<EM> {
    sip: SipServer,
    out_calls: HashMap<InternalCallId, OutgoingCall<EM>>,
    destroy_tx: UnboundedSender<InternalCallId>,
    destroy_rx: UnboundedReceiver<InternalCallId>,
}

impl<EM: EventEmitter> CallManager<EM> {
    pub async fn new(addr: SocketAddr, address_book: AddressBookStorage) -> Self {
        let sip = SipServer::new(addr, address_book).await.expect("should create sip-server");
        let (destroy_tx, destroy_rx) = unbounded_channel();
        Self {
            out_calls: HashMap::new(),
            sip,
            destroy_tx,
            destroy_rx,
        }
    }

    pub fn create_call(&mut self, req: CreateCallRequest) -> Result<CreateCallResponse, CallApiError> {
        let from = format!("sip:{}@{}", req.from_number, req.sip_server);
        let to = format!("sip:{}@{}", req.to_number, req.sip_server);
        match self.sip.make_call(&from, &to, req.sip_auth, req.streaming) {
            Ok(call) => {
                let call_id = call.call_id();
                self.out_calls.insert(call_id.clone(), OutgoingCall::new(call, self.destroy_tx.clone()));
                Ok(CreateCallResponse {
                    ws: format!("/ws/call/{call_id}?token=fake-token-here"),
                    call_id: call_id.clone().into(),
                })
            }
            Err(err) => Err(CallApiError::SipError(err.to_string())),
        }
    }

    pub fn subscribe_call(&mut self, call: InternalCallId, emitter: EM) -> Result<(), CallApiError> {
        let call = self.out_calls.get_mut(&call).ok_or(CallApiError::CallNotFound)?;
        call.add_emitter(emitter);
        Ok(())
    }

    pub fn unsubscribe_call(&mut self, call: InternalCallId, emitter: EmitterId) -> Result<(), CallApiError> {
        let call = self.out_calls.get_mut(&call).ok_or(CallApiError::CallNotFound)?;
        call.del_emitter(emitter);
        Ok(())
    }

    pub fn update_call(&mut self, _call: InternalCallId, _req: UpdateCallRequest) -> Result<UpdateCallResponse, CallApiError> {
        todo!()
    }

    pub fn end_call(&mut self, call: InternalCallId) -> Result<(), CallApiError> {
        let call = self.out_calls.get_mut(&call).ok_or(CallApiError::CallNotFound)?;
        call.end();
        Ok(())
    }

    pub async fn recv(&mut self) -> Option<()> {
        let call_id = self.destroy_rx.recv().await?;
        if self.out_calls.remove(&call_id).is_none() {
            log::warn!("[CallManager] got Destroyed event for {call_id} but not found");
        }
        Some(())
    }
}
