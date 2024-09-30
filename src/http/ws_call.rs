use super::HttpCommand;
use poem::{
    handler,
    web::{websocket::WebSocket, Data, Path},
    IntoResponse,
};
use tokio::sync::mpsc::Sender;

#[handler]
pub fn ws_single_call(
    Path(call_id): Path<String>,
    ws: WebSocket,
    data: Data<&Sender<HttpCommand>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {})
}
