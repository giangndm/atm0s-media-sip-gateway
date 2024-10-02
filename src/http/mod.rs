use std::{io, net::SocketAddr};

use crate::{
    call_manager::EmitterId,
    protocol::{CallApiError, CreateCallRequest, CreateCallResponse, InternalCallId, UpdateCallRequest, UpdateCallResponse},
};
use poem::{get, listener::TcpListener, EndpointExt, Route, Server};
use poem_openapi::OpenApiService;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    oneshot,
};

mod api_call;
mod header_xapi_key;
mod response_result;
mod ws_call;

pub use ws_call::WebsocketEventEmitter;

pub enum HttpCommand {
    CreateCall(CreateCallRequest, oneshot::Sender<Result<CreateCallResponse, CallApiError>>),
    UpdateCall(InternalCallId, UpdateCallRequest, oneshot::Sender<Result<UpdateCallResponse, CallApiError>>),
    EndCall(InternalCallId, oneshot::Sender<Result<(), CallApiError>>),
    SubscribeCall(InternalCallId, WebsocketEventEmitter, oneshot::Sender<Result<(), CallApiError>>),
    UnsubscribeCall(InternalCallId, EmitterId, oneshot::Sender<Result<(), CallApiError>>),
}

pub struct HttpServer {
    addr: SocketAddr,
    secret: String,
    tx: Sender<HttpCommand>,
}

impl HttpServer {
    pub fn new(addr: SocketAddr, secret: &str) -> (Self, Receiver<HttpCommand>) {
        let (tx, rx) = channel(10);
        (Self { addr, secret: secret.to_owned(), tx }, rx)
    }

    pub async fn run_loop(&mut self) -> io::Result<()> {
        let call_service: OpenApiService<_, ()> = OpenApiService::new(
            api_call::CallApis {
                tx: self.tx.clone(),
                secret: self.secret.clone(),
            },
            "Console call APIs",
            env!("CARGO_PKG_VERSION"),
        )
        .server("/")
        .url_prefix("/call");
        let call_ui = call_service.swagger_ui();
        let call_spec = call_service.spec();

        let app = Route::new()
            .nest("/call/", call_service)
            .nest("/docs/call/", call_ui)
            .at("/docs/call/spec", poem::endpoint::make_sync(move |_| call_spec.clone()))
            .at("/ws/call/:call_id", get(ws_call::ws_single_call).data(self.tx.clone()));

        Server::new(TcpListener::bind(self.addr)).run(app).await
    }
}
