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

impl CallbackDataType {
    pub const TYPE_NONE: i32 = 0;
    pub const TYPE_SECRET: i32 = 1;
    pub const TYPE_TEXT: i32 = 2;
    pub const TYPE_BOOLEAN: i32 = 3;

    pub fn original_code(self) -> i32 {
        match self {
            Self::None => Self::TYPE_NONE,
            Self::Secret => Self::TYPE_SECRET,
            Self::Text | Self::Select => Self::TYPE_TEXT,
            Self::Boolean => Self::TYPE_BOOLEAN,
            Self::Unknown(code) => code,
        }
    }

    pub fn from_original_code(code: i32) -> Self {
        match code {
            Self::TYPE_NONE => Self::None,
            Self::TYPE_SECRET => Self::Secret,
            Self::TYPE_TEXT => Self::Text,
            Self::TYPE_BOOLEAN => Self::Boolean,
            code => Self::Unknown(code),
        }
    }
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
    HaveCrcError,
    HaveError,
    HaveIbanError,
    Unknown(i32),
}

impl CallbackReason {
    pub const NEED_COUNTRY: i32 = 7;
    pub const NEED_BLZ: i32 = 8;
    pub const NEED_HOST: i32 = 9;
    pub const NEED_PORT: i32 = 10;
    pub const NEED_USERID: i32 = 11;
    pub const HAVE_INST_MSG: i32 = 14;
    pub const NEED_PT_PIN: i32 = 16;
    pub const NEED_PT_TAN: i32 = 17;
    pub const NEED_CUSTOMERID: i32 = 18;
    pub const HAVE_CRC_ERROR: i32 = 19;
    pub const HAVE_ERROR: i32 = 20;
    pub const NEED_CONNECTION: i32 = 24;
    pub const CLOSE_CONNECTION: i32 = 25;
    pub const NEED_FILTER: i32 = 26;
    pub const NEED_PT_SECMECH: i32 = 27;
    pub const HAVE_IBAN_ERROR: i32 = 30;
    pub const NEED_PT_TANMEDIA: i32 = 32;

    pub fn original_code(self) -> i32 {
        match self {
            Self::NeedCountry => Self::NEED_COUNTRY,
            Self::NeedBlz => Self::NEED_BLZ,
            Self::NeedHost => Self::NEED_HOST,
            Self::NeedPort => Self::NEED_PORT,
            Self::NeedFilter => Self::NEED_FILTER,
            Self::NeedUserId => Self::NEED_USERID,
            Self::NeedCustomerId => Self::NEED_CUSTOMERID,
            Self::NeedPtPin => Self::NEED_PT_PIN,
            Self::NeedPtTan => Self::NEED_PT_TAN,
            Self::NeedPtSecMech => Self::NEED_PT_SECMECH,
            Self::NeedPtTanMedia => Self::NEED_PT_TANMEDIA,
            Self::NeedConnection => Self::NEED_CONNECTION,
            Self::CloseConnection => Self::CLOSE_CONNECTION,
            Self::HaveInstMsg => Self::HAVE_INST_MSG,
            Self::HaveCrcError => Self::HAVE_CRC_ERROR,
            Self::HaveError => Self::HAVE_ERROR,
            Self::HaveIbanError => Self::HAVE_IBAN_ERROR,
            Self::Unknown(code) => code,
        }
    }

    pub fn from_original_code(code: i32) -> Self {
        match code {
            Self::NEED_COUNTRY => Self::NeedCountry,
            Self::NEED_BLZ => Self::NeedBlz,
            Self::NEED_HOST => Self::NeedHost,
            Self::NEED_PORT => Self::NeedPort,
            Self::NEED_FILTER => Self::NeedFilter,
            Self::NEED_USERID => Self::NeedUserId,
            Self::NEED_CUSTOMERID => Self::NeedCustomerId,
            Self::NEED_PT_PIN => Self::NeedPtPin,
            Self::NEED_PT_TAN => Self::NeedPtTan,
            Self::NEED_PT_SECMECH => Self::NeedPtSecMech,
            Self::NEED_PT_TANMEDIA => Self::NeedPtTanMedia,
            Self::NEED_CONNECTION => Self::NeedConnection,
            Self::CLOSE_CONNECTION => Self::CloseConnection,
            Self::HAVE_INST_MSG => Self::HaveInstMsg,
            Self::HAVE_CRC_ERROR => Self::HaveCrcError,
            Self::HAVE_ERROR => Self::HaveError,
            Self::HAVE_IBAN_ERROR => Self::HaveIbanError,
            code => Self::Unknown(code),
        }
    }
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
