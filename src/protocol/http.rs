use poem_openapi::{Enum, Object};
use serde::{Deserialize, Serialize};

use super::StreamingInfo;

#[derive(Debug, Enum, Deserialize)]
pub enum CallAction {
    Trying,
    Ring,
    Accept,
    Reject,
}

#[derive(Debug, Object, Deserialize)]
pub struct CallActionRequest {
    pub action: CallAction,
    pub stream: Option<StreamingInfo>,
}

#[derive(Debug, Object, Serialize)]
pub struct CallActionResponse {}
