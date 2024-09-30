use std::{collections::HashMap, io, net::SocketAddr};

use address_book::AddressBookStorage;
use futures::select2;
use http::HttpServer;
use protocol::{
    CallApiError, CreateCallRequest, CreateCallResponse, InternalCallId, UpdateCallRequest,
    UpdateCallResponse,
};
use sip::{OutgoingCallControl, SipServer};
use thiserror::Error;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot,
};

pub mod address_book;
pub mod futures;
pub mod hook;
pub mod http;
pub mod protocol;
pub mod sip;

#[derive(Error, Debug)]
pub enum GatewayError {
    #[error("IoError {0}")]
    Io(#[from] io::Error),
    #[error("SipError {0}")]
    Sip(#[from] sip::SipServerError),
    #[error("QueueError")]
    Queue,
}

enum InternalControl {
    CallDestroyed(InternalCallId),
}

pub struct Gateway {
    http: SocketAddr,
    secret: String,
    sip: SipServer,
    internal_control_tx: Sender<InternalControl>,
    internal_control_rx: Receiver<InternalControl>,
    out_calls: HashMap<InternalCallId, Sender<OutgoingCallControl>>,
}

impl Gateway {
    pub async fn new(
        http: SocketAddr,
        secret: &str,
        sip: SocketAddr,
        address_book: AddressBookStorage,
    ) -> Result<Self, GatewayError> {
        let sip = SipServer::new(sip, address_book).await?;

        let (internal_control_tx, internal_control_rx) = channel(10);

        Ok(Self {
            http,
            secret: secret.to_owned(),
            sip,
            internal_control_tx,
            internal_control_rx,
            out_calls: HashMap::new(),
        })
    }

    async fn create_call(
        &mut self,
        req: CreateCallRequest,
        tx: oneshot::Sender<Result<CreateCallResponse, CallApiError>>,
    ) {
        let from = format!("sip:{}@{}", req.from_number, req.sip_server);
        let to = format!("sip:{}@{}", req.to_number, req.sip_server);
        match self.sip.make_call(&from, &to, req.sip_auth) {
            Ok(mut call) => {
                let call_id = call.internal_call_id();
                match tx.send(Ok(CreateCallResponse {
                    ws: format!("/ws/call/{call_id}?token=fake-token-here"),
                    call_id: call_id.clone().into(),
                })) {
                    Ok(_) => {
                        self.out_calls.insert(
                            call_id.clone(),
                            call.take_control_tx().expect("Should have control_tx"),
                        );
                        let internal_control_tx = self.internal_control_tx.clone();
                        tokio::spawn(async move {
                            log::info!("[Gateway] start loop for call {call_id}");
                            match call.run_loop().await {
                                Ok(_) => {
                                    log::info!("[Gateway] end loop for call {call_id}");
                                }
                                Err(err) => {
                                    log::error!("[Gateway] error loop for call {call_id}, {err:?}");
                                }
                            }
                            internal_control_tx
                                .send(InternalControl::CallDestroyed(call_id))
                                .await
                                .expect("Should send to main loop");
                        });
                    }
                    Err(err) => {
                        log::error!(
                            "[Gateway] send response for creating call error {err:?} => drop call"
                        );
                    }
                }
            }
            Err(err) => {
                if let Err(err) = tx.send(Err(CallApiError::InternalChannel(err.to_string()))) {
                    log::error!("[Gateway] send error response for creating call error {err:?}");
                }
            }
        }
    }

    async fn update_call(
        &mut self,
        call_id: InternalCallId,
        req: UpdateCallRequest,
        tx: oneshot::Sender<Result<UpdateCallResponse, CallApiError>>,
    ) {
        todo!()
    }

    async fn end_call(
        &mut self,
        call_id: InternalCallId,
        tx: oneshot::Sender<Result<(), CallApiError>>,
    ) {
        if let Some(control_tx) = self.out_calls.get(&call_id) {
            control_tx.send(OutgoingCallControl::End).await;
            tx.send(Ok(()));
        } else {
            log::warn!("[Gateway] call not found {call_id}");
            tx.send(Err(CallApiError::CallNotFound));
        }
    }

    pub async fn run_loop(&mut self) -> Result<(), GatewayError> {
        let (mut http, mut http_rx) = HttpServer::new(self.http, &self.secret);
        tokio::spawn(async move { http.run_loop().await });
        // TODO join with http task

        loop {
            let out = select2::or(http_rx.recv(), self.internal_control_rx.recv()).await;
            match out {
                select2::OrOutput::Left(http_event) => {
                    match http_event.ok_or(GatewayError::Queue)? {
                        http::HttpCommand::CreateCall(req, tx) => {
                            self.create_call(req, tx).await;
                        }
                        http::HttpCommand::UpdateCall(call_id, req, tx) => {
                            self.update_call(call_id, req, tx).await;
                        }
                        http::HttpCommand::EndCall(call_id, tx) => {
                            self.end_call(call_id, tx).await;
                        }
                    }
                }
                select2::OrOutput::Right(control_event) => {
                    match control_event.ok_or(GatewayError::Queue)? {
                        InternalControl::CallDestroyed(internal_call_id) => {
                            log::info!("[Gateway] call {internal_call_id} destroyed");
                            self.out_calls.remove(&internal_call_id);
                        }
                    }
                }
            }
        }
    }
}
