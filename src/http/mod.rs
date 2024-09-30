use std::{io, net::SocketAddr};

use poem::{get, listener::TcpListener, Route, Server};
use poem_openapi::OpenApiService;

mod api_call;
mod header_xapi_key;
mod response_result;
mod ws_call;

pub struct HttpServer {
    addr: SocketAddr,
}

impl HttpServer {
    pub fn new(addr: SocketAddr) -> Self {
        Self { addr }
    }

    pub async fn run_loop(&mut self) -> io::Result<()> {
        let call_service: OpenApiService<_, ()> = OpenApiService::new(
            api_call::CallApis {},
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
            .at(
                "/docs/call/spec",
                poem::endpoint::make_sync(move |_| call_spec.clone()),
            )
            .at("/ws/call/:call_id", get(ws_call::ws_single_call));

        Server::new(TcpListener::bind(self.addr)).run(app).await
    }
}
