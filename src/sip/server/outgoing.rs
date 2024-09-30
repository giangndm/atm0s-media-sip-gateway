use ezk_sip_auth::{
    digest::{DigestAuthenticator, DigestCredentials},
    CredentialStore, UacAuthSession,
};
use ezk_sip_core::{Endpoint, LayerKey};
use ezk_sip_types::{header::typed::Contact, uri::NameAddr};
use ezk_sip_ua::{
    dialog::DialogLayer,
    invite::{
        initiator::{Early, Initiator},
        session::Session,
        InviteLayer,
    },
};
use thiserror::Error;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    futures::select2,
    protocol::{InternalCallId, SipAuth},
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
pub enum OutgoingCallError {
    #[error("EzkCoreError({0})")]
    EzkCore(#[from] ezk_sip_core::Error),
    #[error("EzkAuthError({0})")]
    EzkAuth(#[from] ezk_sip_auth::Error),
    #[error("SipError({0})")]
    Sip(u16),
    #[error("InternalChannel")]
    InternalChannel,
}

pub enum OutgoingCallControl {
    End,
}

pub struct OutgoingCall {
    internal_call_id: InternalCallId,
    state: InternalState,
    initiator: Initiator,
    auth: Option<OutgoingAuth>,
    control_tx: Option<Sender<OutgoingCallControl>>,
    control_rx: Receiver<OutgoingCallControl>,
}

impl OutgoingCall {
    pub fn new(
        endpoint: Endpoint,
        dialog_layer: LayerKey<DialogLayer>,
        invite_layer: LayerKey<InviteLayer>,
        from: &str,
        to: &str,
        auth: Option<SipAuth>,
    ) -> Result<Self, OutgoingCallError> {
        let internal_call_id: InternalCallId = InternalCallId::random();
        log::info!("[OutgoingCall {internal_call_id}] create with {from} => {to}");
        let local_uri = endpoint.parse_uri(from).unwrap();
        let target = endpoint.parse_uri(to).unwrap();

        let initiator = Initiator::new(
            endpoint,
            dialog_layer,
            invite_layer,
            NameAddr::uri(local_uri.clone()),
            Contact::new(NameAddr::uri(local_uri)),
            target,
        );

        let auth = auth.map(|auth| {
            let mut credentials = CredentialStore::new();
            credentials.set_default(DigestCredentials::new(auth.username, auth.password));
            OutgoingAuth {
                session: UacAuthSession::new(DigestAuthenticator::default()),
                credentials,
            }
        });

        let (control_tx, control_rx) = channel(10);

        Ok(Self {
            initiator,
            state: InternalState::Calling { auth_failed: false },
            auth,
            internal_call_id,
            control_tx: Some(control_tx),
            control_rx,
        })
    }

    pub fn take_control_tx(&mut self) -> Option<Sender<OutgoingCallControl>> {
        self.control_tx.take()
    }

    pub fn internal_call_id(&self) -> InternalCallId {
        self.internal_call_id.clone()
    }

    pub async fn run_loop(&mut self) -> Result<(), OutgoingCallError> {
        let invite = self.initiator.create_invite();
        self.initiator.send_invite(invite).await?;
        let call_id = self.internal_call_id();
        log::info!("[OutgoingCall {call_id}] start loop");

        let res = loop {
            let select = select2::or(self.initiator.receive(), self.control_rx.recv()).await;

            match select {
                select2::OrOutput::Left(event) => match event? {
                    ezk_sip_ua::invite::initiator::Response::Provisional(response) => {
                        let code = response.line.code.into_u16();
                        log::info!("[OutgoingCall {call_id}] on Provisional {code}");
                    }
                    ezk_sip_ua::invite::initiator::Response::Failure(response) => {
                        if let InternalState::Calling { auth_failed } = &mut self.state {
                            if *auth_failed {
                                log::error!(
                                    "[OutgoingCall {call_id}] already has authen error => reject"
                                );
                                break Err(OutgoingCallError::Sip(response.line.code.into_u16()));
                            } else if response.line.code.into_u16() == 401 {
                                *auth_failed = true;
                                //unauth processing
                                if let Some(auth) = &mut self.auth {
                                    log::info!("[OutgoingCall {call_id}] resend invite with auth");
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

                                    let mut invite = self.initiator.create_invite();
                                    auth.session.authorize_request(&mut invite.headers);
                                    self.initiator.send_invite(invite).await?;
                                } else {
                                    log::error!("[OutgoingCall {call_id}] call auth required");
                                    break Err(OutgoingCallError::Sip(
                                        response.line.code.into_u16(),
                                    ));
                                }
                            } else {
                                log::error!(
                                    "[OutgoingCall {call_id}] call failed {}",
                                    response.line.code.into_u16()
                                );
                                break Err(OutgoingCallError::Sip(response.line.code.into_u16()));
                            }
                        } else {
                            log::error!(
                                "[OutgoingCall {call_id}] call failed {}",
                                response.line.code.into_u16()
                            );
                            break Err(OutgoingCallError::Sip(response.line.code.into_u16()));
                        }
                    }
                    ezk_sip_ua::invite::initiator::Response::Early(early, tsx, req) => {
                        log::info!("[OutgoingCall {call_id}] on early");
                        self.state = InternalState::Early { early };
                    }
                    ezk_sip_ua::invite::initiator::Response::Session(session, _response) => {
                        log::info!("[OutgoingCall {call_id}] call establisted");
                        self.state = InternalState::Talking { session };
                    }
                    ezk_sip_ua::invite::initiator::Response::Finished => {
                        log::info!("[OutgoingCall {call_id}] call finished");
                        self.state = InternalState::Destroyed;
                        break Ok(());
                    }
                },
                select2::OrOutput::Right(event) => {
                    match event.ok_or(OutgoingCallError::InternalChannel)? {
                        OutgoingCallControl::End => {
                            log::info!("end call");
                            break Ok(());
                        }
                    }
                }
            }
        };
        log::info!("[OutgoingCall {call_id}] end loop {res:?}");
        res
    }
}
