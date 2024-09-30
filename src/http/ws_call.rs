use poem::{
    handler,
    web::{websocket::WebSocket, Path},
    IntoResponse,
};

#[handler]
pub fn ws_single_call(Path(call_id): Path<String>, ws: WebSocket) -> impl IntoResponse {
    ws.on_upgrade(move |socket| async move {})
}
