use poem_openapi::{param::Path, payload::Json, OpenApi};
use tokio::sync::{mpsc::Sender, oneshot};

use crate::protocol::{CallApiError, CreateCallRequest, CreateCallResponse, UpdateCallRequest, UpdateCallResponse};

use super::{header_xapi_key::HeaderXApiKey, response_result::ApiRes, HttpCommand};

pub struct CallApis {
    pub secret: String,
    pub tx: Sender<HttpCommand>,
}

#[OpenApi]
impl CallApis {
    #[oai(path = "/", method = "post")]
    async fn create_call(&self, secret: HeaderXApiKey, data: Json<CreateCallRequest>) -> ApiRes<CreateCallResponse, CallApiError> {
        if self.secret != secret.0.key {
            return Err(CallApiError::WrongSecret.into());
        }

        let (tx, rx) = oneshot::channel();
        self.tx.send(HttpCommand::CreateCall(data.0, tx)).await.map_err(|e| CallApiError::InternalChannel(e.to_string()))?;

        let res = rx.await.map_err(|e| CallApiError::InternalChannel(e.to_string()))??;
        Ok(res.into())
    }

    #[oai(path = "/:call_id", method = "put")]
    async fn update_call(&self, secret: HeaderXApiKey, Path(call): Path<String>, data: Json<UpdateCallRequest>) -> ApiRes<UpdateCallResponse, CallApiError> {
        if self.secret != secret.0.key {
            return Err(CallApiError::WrongSecret.into());
        }

        let (tx, rx) = oneshot::channel();
        self.tx
            .send(HttpCommand::UpdateCall(call.into(), data.0, tx))
            .await
            .map_err(|e| CallApiError::InternalChannel(e.to_string()))?;

        let res = rx.await.map_err(|e| CallApiError::InternalChannel(e.to_string()))??;
        Ok(res.into())
    }

    #[oai(path = "/:call_id", method = "delete")]
    async fn encode_call(&self, secret: HeaderXApiKey, Path(call_id): Path<String>) -> ApiRes<String, CallApiError> {
        if self.secret != secret.0.key {
            return Err(CallApiError::WrongSecret.into());
        }

        let (tx, rx) = oneshot::channel();
        self.tx.send(HttpCommand::EndCall(call_id.into(), tx)).await.map_err(|e| CallApiError::InternalChannel(e.to_string()))?;

        rx.await.map_err(|e| CallApiError::InternalChannel(e.to_string()))??;
        Ok("OK".to_string().into())
    }
}
