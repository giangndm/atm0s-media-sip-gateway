use poem_openapi::{param::Path, payload::Json, Object, OpenApi};
use thiserror::Error;

use crate::protocol::{OutgoingCallAction, SipAuth, StreamingInfo};

use super::{header_xapi_key::HeaderXApiKey, response_result::ApiRes};

#[derive(Debug, Object)]
pub struct CreateCallRequest {
    sip_server: String,
    sip_auth: SipAuth,
    from_number: String,
    to_number: String,
    hook: String,
    streaming: StreamingInfo,
}

#[derive(Debug, Object)]
pub struct CreateCallResponse {
    call_id: String,
    ws: String,
}

#[derive(Debug, Object)]
pub struct UpdateCallRequest {
    action: OutgoingCallAction,
}

#[derive(Debug, Object)]
pub struct UpdateCallResponse {}

#[derive(Error, Debug)]
pub enum CallApiError {
    #[error("Unknown")]
    Unknown,
}

pub struct CallApis {}

#[OpenApi]
impl CallApis {
    #[oai(path = "/", method = "post")]
    async fn create_call(
        &self,
        secret: HeaderXApiKey,
        data: Json<CreateCallRequest>,
    ) -> ApiRes<CreateCallResponse, CallApiError> {
        todo!()
    }

    #[oai(path = "/:call_id", method = "put")]
    async fn update_call(
        &self,
        secret: HeaderXApiKey,
        Path(call): Path<String>,
        data: Json<UpdateCallRequest>,
    ) -> ApiRes<UpdateCallResponse, CallApiError> {
        todo!()
    }
}
