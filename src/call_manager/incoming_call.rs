use std::collections::HashMap;

use anyhow::anyhow;
use tokio::sync::{
    mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender},
    oneshot,
};

use crate::{
    error::PrintErrorDetails,
    futures::select2,
    hook::HttpHookSender,
    protocol::{CallAction, CallActionRequest, HookIncomingCallRequest, HookIncomingCallResponse, IncomingCallEvent, InternalCallId},
    sip::{MediaApi, SipIncomingCall, SipIncomingCallOut},
};

use super::{EmitterId, EventEmitter};

pub struct IncomingCall<EM> {
    control_tx: UnboundedSender<CallControl<EM>>,
}

impl<EM: EventEmitter> IncomingCall<EM> {
    pub fn new(api: MediaApi, sip: SipIncomingCall, destroy_tx: UnboundedSender<InternalCallId>, hook: HttpHookSender) -> Self {
        let (control_tx, control_rx) = unbounded_channel();
        tokio::spawn(async move {
            let call_id = sip.call_id();
            if let Err(e) = run_call_loop(api, sip, control_rx, hook).await {
                log::error!("[IncomingCall] call {call_id} error {e:?}");
            }
            destroy_tx.send(call_id).expect("should send destroy request to main loop");
        });

        Self { control_tx }
    }

    pub fn add_emitter(&mut self, emitter: EM) {
        if let Err(e) = self.control_tx.send(CallControl::Sub(emitter)) {
            log::error!("[IncomingCall] send Sub control error {e:?}");
        }
    }

    pub fn del_emitter(&mut self, emitter: EmitterId) {
        if let Err(e) = self.control_tx.send(CallControl::Unsub(emitter)) {
            log::error!("[IncomingCall] send Unsub control error {e:?}");
        }
    }

    pub fn do_action(&mut self, action: CallActionRequest, tx: oneshot::Sender<anyhow::Result<()>>) {
        if let Err(e) = self.control_tx.send(CallControl::Action(action, tx)) {
            log::error!("[IncomingCall] send Unsub control error {e:?}");
        }
    }

    pub fn end(&mut self) {
        if let Err(e) = self.control_tx.send(CallControl::End) {
            log::error!("[IncomingCall] send End control error {e:?}");
        }
    }
}

enum CallControl<EM> {
    Sub(EM),
    Unsub(EmitterId),
    Action(CallActionRequest, oneshot::Sender<anyhow::Result<()>>),
    End,
}

async fn run_call_loop<EM: EventEmitter>(api: MediaApi, mut call: SipIncomingCall, mut control_rx: UnboundedReceiver<CallControl<EM>>, hook: HttpHookSender) -> anyhow::Result<()> {
    let call_id = call.call_id();
    let from = call.from().to_owned();
    let to = call.to().to_owned();

    let mut emitters: HashMap<EmitterId, EM> = HashMap::new();

    log::info!("[IncomingCall] call starting");

    // we send trying first
    call.send_trying().await?;

    // feedback hook for info
    let res: HookIncomingCallResponse = hook.request(&HookIncomingCallRequest { call_id, from, to }).await?;

    match res.action {
        CallAction::Trying => {}
        CallAction::Ring => call.send_ringing().await?,
        CallAction::Reject => {
            call.kill_because_validate_failed();
            return Ok(());
        }
        CallAction::Accept => {
            let stream = res.stream.ok_or(anyhow!("missing stream in accept action"))?;
            call.accept(api.clone(), stream).await?;
        }
    };

    log::info!("[IncomingCall] call started");

    loop {
        let out = select2::or(call.recv(), control_rx.recv()).await;
        match out {
            select2::OrOutput::Left(Ok(Some(out))) => match out {
                SipIncomingCallOut::Event(event) => {
                    for emitter in emitters.values_mut() {
                        emitter.fire(&event);
                    }
                    hook.send(&event);
                }
                SipIncomingCallOut::Continue => {}
            },
            select2::OrOutput::Left(Ok(None)) => {
                log::info!("[IncomingCall] call end");
                break;
            }
            select2::OrOutput::Left(Err(e)) => {
                log::error!("[IncomingCall] call error {e:?}");
                let event = IncomingCallEvent::Error { message: e.to_string() };

                for emitter in emitters.values_mut() {
                    emitter.fire(&event);
                }
                hook.send(&event);
                break;
            }
            select2::OrOutput::Right(Some(control)) => match control {
                CallControl::Sub(emitter) => {
                    emitters.insert(emitter.emitter_id(), emitter);
                }
                CallControl::Unsub(emitter_id) => {
                    if emitters.remove(&emitter_id).is_some() {
                        if emitters.is_empty() {
                            log::info!("[IncomingCall] all sub disconnected => end call");
                            if let Err(e) = call.end().await {
                                log::error!("[IncomingCall] end call error {e:?}");
                            }
                            break;
                        }
                    }
                }
                CallControl::End => {
                    log::info!("[IncomingCall] received end request");
                    if let Err(e) = call.end().await {
                        log::error!("[IncomingCall] end call error {e:?}");
                    }
                    break;
                }
                CallControl::Action(action, tx) => {
                    let res = match action.action {
                        CallAction::Trying => call.send_trying().await.map_err(|e| e.into()),
                        CallAction::Ring => call.send_ringing().await.map_err(|e| e.into()),
                        CallAction::Reject => call.end().await.map_err(|e| e.into()),
                        CallAction::Accept => {
                            if let Some(stream) = action.stream {
                                call.accept(api.clone(), stream).await.map_err(|e| e.into())
                            } else {
                                Err(anyhow!("missing stream in accept action"))
                            }
                        }
                    };
                    tx.send(res).print_error_detail("[IncomingCall] send action res");
                }
            },
            select2::OrOutput::Right(None) => {
                break;
            }
        }
    }

    log::info!("[IncomingCall] call destroyed");
    let event = IncomingCallEvent::Destroyed;
    for emitter in emitters.values_mut() {
        emitter.fire(&event);
    }
    hook.send(&event);
    Ok(())
}
