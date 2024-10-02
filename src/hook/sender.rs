use std::collections::HashMap;

use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;

use super::queue::HttpHookRequest;

pub struct HttpHookSender {
    pub endpoint: String,
    pub headers: HashMap<String, String>,
    pub tx: UnboundedSender<HttpHookRequest>,
}

impl HttpHookSender {
    pub fn send<E: Serialize>(&self, body: &E) {
        self.tx
            .send(HttpHookRequest {
                endpoint: self.endpoint.clone(),
                headers: self.headers.clone(),
                body: serde_json::to_vec(body).expect("should convert to json").into(),
            })
            .expect("should send to queue worker");
    }
}
