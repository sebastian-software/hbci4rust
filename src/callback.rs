use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::HbciResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallbackDataType {
    None,
    Text,
    Secret,
    Boolean,
    Select,
    Unknown(i32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallbackReason {
    NeedCountry,
    NeedBlz,
    NeedHost,
    NeedPort,
    NeedFilter,
    NeedUserId,
    NeedCustomerId,
    NeedPtPin,
    NeedPtTan,
    NeedPtSecMech,
    NeedPtTanMedia,
    NeedConnection,
    CloseConnection,
    HaveInstMsg,
    HaveError,
    Unknown(i32),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallbackEvent {
    pub reason: CallbackReason,
    pub message: String,
    pub data_type: CallbackDataType,
    pub current_value: Option<String>,
}

impl CallbackEvent {
    pub fn new(reason: CallbackReason) -> Self {
        Self {
            reason,
            message: String::new(),
            data_type: CallbackDataType::None,
            current_value: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CallbackResponse {
    pub value: Option<String>,
    pub accepted: bool,
}

impl CallbackResponse {
    pub fn empty() -> Self {
        Self {
            value: None,
            accepted: true,
        }
    }

    pub fn value(value: impl Into<String>) -> Self {
        Self {
            value: Some(value.into()),
            accepted: true,
        }
    }
}

#[async_trait]
pub trait HbciCallback: Send + Sync {
    async fn handle(&self, event: CallbackEvent) -> HbciResult<CallbackResponse>;
}
