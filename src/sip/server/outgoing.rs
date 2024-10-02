use ezk_sip_auth::{
    digest::{DigestAuthenticator, DigestCredentials},
    CredentialStore, UacAuthSession,
};
use ezk_sip_core::{Endpoint, LayerKey};
use ezk_sip_types::{header::typed::Contact, uri::NameAddr};
use ezk_sip_ua::{
    dialog::DialogLayer,
    invite::{
        initiator::{Early, EarlyResponse, Initiator, Response},
        session::Session,
        InviteLayer,
    },
};
use thiserror::Error;

use crate::{
    futures::select2,
    protocol::{InternalCallId, OutgoingCallEvent, SipAuth, StreamingInfo},
    sip::{RtpEngineError, RtpEngineOffer},
};

struct OutgoingAuth {
    session: UacAuthSession,
    credentials: CredentialStore,
}

enum InternalState {
    Calling { auth_failed: bool },
    Early { early: Early },
    Talking { session: Session },
    Destroyed,
}

#[derive(Error, Debug)]
pub enum SipOutgoingCallError {
    #[error("EzkCoreError({0})")]
    EzkCore(#[from] ezk_sip_core::Error),
    #[error("EzkAuthError({0})")]
    EzkAuth(#[from] ezk_sip_auth::Error),
    #[error("SipError({0})")]
    Sip(u16),
    #[error("InternalChannel")]
    InternalChannel,
    #[error("RtpEngine{0}")]
    RtpEngine(#[from] RtpEngineError),
}

pub enum SipOutgoingCallOut {
    Event(OutgoingCallEvent),
    Continue,
}

pub struct SipOutgoingCall {
    call_id: InternalCallId,
    state: InternalState,
    initiator: Initiator,
    auth: Option<OutgoingAuth>,
    rtp: RtpEngineOffer,
}

impl SipOutgoingCall {
    pub fn new(
        endpoint: Endpoint,
        dialog_layer: LayerKey<DialogLayer>,
        invite_layer: LayerKey<InviteLayer>,
        from: &str,
        to: &str,
        auth: Option<SipAuth>,
        stream: StreamingInfo,
    ) -> Result<Self, SipOutgoingCallError> {
        let call_id: InternalCallId = InternalCallId::random();
        log::info!("[SipOutgoingCall {call_id}] create with {from} => {to}");
        let local_uri = endpoint.parse_uri(from).unwrap();
        let target = endpoint.parse_uri(to).unwrap();

        let initiator = Initiator::new(endpoint, dialog_layer, invite_layer, NameAddr::uri(local_uri.clone()), Contact::new(NameAddr::uri(local_uri)), target);

        let auth = auth.map(|auth| {
            let mut credentials = CredentialStore::new();
            credentials.set_default(DigestCredentials::new(auth.username, auth.password));
            OutgoingAuth {
                session: UacAuthSession::new(DigestAuthenticator::default()),
                credentials,
            }
        });

        Ok(Self {
            initiator,
            state: InternalState::Calling { auth_failed: false },
            auth,
            call_id,
            rtp: RtpEngineOffer::new(&stream.gateway, &stream.token),
        })
    }

    pub fn call_id(&self) -> InternalCallId {
        self.call_id.clone()
    }

    pub async fn start(&mut self) -> Result<(), SipOutgoingCallError> {
        if self.rtp.sdp().is_none() {
            self.rtp.create_offer().await?;
        }

        let sdp = self.rtp.sdp().expect("should have sdp");
        let mut invite = self.initiator.create_invite();
        invite.body = sdp.clone();
        if let Some(auth) = &mut self.auth {
            auth.session.authorize_request(&mut invite.headers);
        }

        self.initiator.send_invite(invite).await?;
        let call_id = self.call_id();
        log::info!("[SipOutgoingCall {call_id}] start loop with sdp {}", String::from_utf8_lossy(&sdp));
        Ok(())
    }

    pub async fn end(&mut self) -> Result<(), SipOutgoingCallError> {
        match &mut self.state {
            InternalState::Talking { session } => {
                session.terminate().await?;
                Ok(())
            }
            InternalState::Destroyed => Ok(()),
            _ => {
                let mut cancel = self.initiator.create_cancel();
                if let Some(auth) = &mut self.auth {
                    auth.session.authorize_request(&mut cancel.headers);
                }
                self.initiator.send_cancel(cancel).await?;
                Ok(())
            }
        }
    }

    pub async fn recv(&mut self) -> Result<Option<SipOutgoingCallOut>, SipOutgoingCallError> {
        let call_id = self.call_id();
        let out = if let InternalState::Early { early } = &mut self.state {
            select2::or(self.initiator.receive(), early.receive()).await
        } else {
            select2::OrOutput::Left(self.initiator.receive().await)
        };

        match out {
            select2::OrOutput::Left(main_event) => match main_event? {
                Response::Provisional(response) => {
                    let code = response.line.code.into_u16();
                    log::info!("[SipOutgoingCall {call_id}] on Provisional {code}");
                    Ok(Some(SipOutgoingCallOut::Event(OutgoingCallEvent::Provisional { code })))
                }
                Response::Failure(response) => {
                    if let InternalState::Calling { auth_failed } = &mut self.state {
                        if response.line.code.into_u16() != 401 {
                            log::error!("[SipOutgoingCall {call_id}] error => reject");
                            Err(SipOutgoingCallError::Sip(response.line.code.into_u16()))
                        } else if response.line.code.into_u16() == 401 {
                            //unauth processing
                            if let Some(auth) = &mut self.auth {
                                if *auth_failed {
                                    Err(SipOutgoingCallError::Sip(response.line.code.into_u16()))
                                } else {
                                    *auth_failed = true;
                                    log::info!("[SipOutgoingCall {call_id}] resend invite with auth");
                                    let tsx = self.initiator.transaction().unwrap();
                                    let inv = tsx.request();

                                    auth.session.handle_authenticate(
                                        &response.headers,
                                        &auth.credentials,
                                        ezk_sip_auth::RequestParts {
                                            line: &inv.msg.line,
                                            headers: &inv.msg.headers,
                                            body: b"",
                                        },
                                    )?;

                                    self.start().await?;
                                    Ok(Some(SipOutgoingCallOut::Continue))
                                }
                            } else {
                                log::error!("[SipOutgoingCall {call_id}] call auth required");
                                Err(SipOutgoingCallError::Sip(response.line.code.into_u16()))
                            }
                        } else {
                            log::error!("[SipOutgoingCall {call_id}] call failed {}", response.line.code.into_u16());
                            Err(SipOutgoingCallError::Sip(response.line.code.into_u16()))
                        }
                    } else {
                        log::error!("[SipOutgoingCall {call_id}] call failed {}", response.line.code.into_u16());
                        Err(SipOutgoingCallError::Sip(response.line.code.into_u16()))
                    }
                }
                Response::Early(early, response, _req) => {
                    let code = response.line.code.into_u16();
                    log::info!("[SipOutgoingCall {call_id}] on early code: {code}");
                    self.state = InternalState::Early { early };
                    Ok(Some(SipOutgoingCallOut::Event(OutgoingCallEvent::Early { code })))
                }
                Response::Session(session, response) => {
                    let code = response.line.code.into_u16();
                    log::info!("[SipOutgoingCall {call_id}] call establisted code: {code}");
                    if response.body.len() > 0 {
                        self.rtp.set_answer(response.body.clone()).await?;
                    }

                    self.state = InternalState::Talking { session };
                    Ok(Some(SipOutgoingCallOut::Event(OutgoingCallEvent::Accepted { code })))
                }
                Response::Finished => {
                    log::info!("[SipOutgoingCall {call_id}] call finished");
                    self.state = InternalState::Destroyed;
                    Ok(None)
                }
            },
            select2::OrOutput::Right(early_event) => match early_event? {
                EarlyResponse::Provisional(response, _rseq) => {
                    let code = response.line.code.into_u16();
                    log::info!("[SipOutgoingCall {call_id}] early Provisional {code}");
                    Ok(Some(SipOutgoingCallOut::Continue))
                }
                EarlyResponse::Success(_session, response) => {
                    let code = response.line.code.into_u16();
                    log::info!("[SipOutgoingCall {call_id}] early Success {code}");
                    Ok(Some(SipOutgoingCallOut::Continue))
                }
                EarlyResponse::Terminated => {
                    log::info!("[SipOutgoingCall {call_id}] early Terminated");
                    Ok(Some(SipOutgoingCallOut::Continue))
                }
            },
        }
    }
}
