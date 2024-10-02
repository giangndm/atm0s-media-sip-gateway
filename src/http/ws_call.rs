use crate::{
    call_manager::{EmitterId, EventEmitter},
    futures::select2::{self, OrOutput},
    protocol::InternalCallId,
};

use super::HttpCommand;
use futures_util::{SinkExt, StreamExt};
use poem::{
    handler,
    web::{
        websocket::{Message, WebSocket},
        Data, Path,
    },
    IntoResponse,
};
use serde::Serialize;
use tokio::sync::{
    mpsc::{unbounded_channel, Sender, UnboundedSender},
    oneshot,
};

#[handler]
pub fn ws_single_call(Path(call_id): Path<String>, ws: WebSocket, data: Data<&Sender<HttpCommand>>) -> impl IntoResponse {
    let cmd_tx = data.clone();
    ws.on_upgrade(move |socket| async move {
        let (mut sink, mut stream) = socket.split();
        let emitter_id = EmitterId::rand();
        let call_id: InternalCallId = call_id.into();
        let (out_tx, mut out_rx) = unbounded_channel();
        let emitter = WebsocketEventEmitter { emitter_id, out_tx };

        let (tx, rx) = oneshot::channel();
        if let Err(e) = cmd_tx.send(HttpCommand::SubscribeCall(call_id.clone(), emitter, tx)).await {
            log::error!("[WsCall {emitter_id}] send sub_cmd error {e:?}");
            return;
        }

        match rx.await {
            Ok(res) => match res {
                Ok(_) => {}
                Err(err) => {
                    log::error!("[WsCall {emitter_id}] sub_cmd got error {err:?}");
                    return;
                }
            },
            Err(err) => {
                log::error!("[WsCall {emitter_id}] send sub_cmd error {err:?}");
                return;
            }
        }

        loop {
            let out = select2::or(out_rx.recv(), stream.next()).await;
            match out {
                OrOutput::Left(Some(out)) => {
                    if let Err(e) = sink.send(Message::Text(out)).await {
                        log::error!("[WsCall {emitter_id}] send data error {e:?}");
                        break;
                    }
                }
                OrOutput::Left(_) => {
                    break;
                }
                OrOutput::Right(Some(Ok(_))) => {
                    log::info!("[WsCall {emitter_id}] received data");
                }
                OrOutput::Right(_) => {
                    log::info!("[WsCall {emitter_id}] socket closed");
                    break;
                }
            }
        }

        let (tx, rx) = oneshot::channel();
        if let Err(e) = cmd_tx.send(HttpCommand::UnsubscribeCall(call_id.clone(), emitter_id, tx)).await {
            log::error!("[WsCall {emitter_id}] send sub_cmd error {e:?}");
            return;
        }

        match rx.await {
            Ok(res) => match res {
                Ok(_) => {}
                Err(err) => {
                    log::error!("[WsCall {emitter_id}] sub_cmd got error {err:?}");
                    return;
                }
            },
            Err(err) => {
                log::error!("[WsCall {emitter_id}] send sub_cmd error {err:?}");
                return;
            }
        }
    })
}

pub struct WebsocketEventEmitter {
    emitter_id: EmitterId,
    out_tx: UnboundedSender<String>,
}

impl EventEmitter for WebsocketEventEmitter {
    fn emitter_id(&self) -> EmitterId {
        self.emitter_id
    }

    fn fire<E: Serialize>(&mut self, event: &E) {
        let json_str = serde_json::to_string(event).expect("should convert to json");
        if let Err(e) = self.out_tx.send(json_str) {
            log::error!("[WebsocketEventEmitter] send event error {e:?}");
        }
    }
}
