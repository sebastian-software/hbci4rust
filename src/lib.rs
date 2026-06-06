//! Original-near Rust port scaffold for hbci4java.
//!
//! The crate keeps hbci4java concepts recognizable while choosing Rust-shaped
//! public type names and async control flow.

pub mod callback;
pub mod comm;
pub mod dialog;
pub mod error;
pub mod gv;
pub mod gv_result;
pub mod manager;
pub mod passport;
pub mod protocol;
pub mod sepa;
pub mod swift;
pub mod tools;

pub use callback::{
    CallbackDataType, CallbackEvent, CallbackReason, CallbackResponse, HbciCallback,
};
pub use comm::{CommClient, CommRequest, CommResponse, DefaultCommClient, ReplayCommClient};
pub use dialog::{DialogContext, KnownReturncode};
pub use error::{HbciError, HbciErrorKind, HbciResult};
pub use gv::{HbciJob, HbciJobConstraint, JobRegistry, PINTAN_JOB_NAMES};
pub use gv_result::{
    GvrKUms, GvrKUmsBTag, GvrKUmsLine, GvrSaldoReq, GvrSaldoReqInfo, HbciDialogStatus,
    HbciExecStatus, HbciInstMessage, HbciJobResult, HbciJobResultData, HbciMsgStatus,
    HbciReturnValue, HbciStatus, HbciStatusCode, Konto, Limit, Saldo, Value,
};
pub use manager::{
    AccountCrcAlgs, BankInfo, BankInfoRegistry, ChallengeHhdVersion, ChallengeInfo, ChallengeJob,
    ChallengeParam, FlickerCode, FlickerCodeVersion, FlickerDataElement, FlickerEncoding,
    FlickerRenderer, FlickerStartCode, HbciHandler, HbciVersion, HhdVersion, HhdVersionType,
    MatrixCode, QrCode, done, get_param, init, set_param,
};
pub use passport::{PassportStorage, PinTanPassport, PinTanPassportData};
pub use tools::{
    ParameterFinder, ParameterQuery, Properties, has_text, join_strings, safe_filename, to_boolean,
    to_ins_code, to_parameter_code,
};
