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

pub use callback::{
    CallbackDataType, CallbackEvent, CallbackReason, CallbackResponse, HbciCallback,
};
pub use comm::{CommClient, CommRequest, CommResponse, DefaultCommClient, ReplayCommClient};
pub use dialog::DialogContext;
pub use error::{HbciError, HbciErrorKind, HbciResult};
pub use gv::{HbciJob, JobRegistry, PINTAN_JOB_NAMES};
pub use gv_result::{
    GvrSaldoReq, GvrSaldoReqInfo, HbciExecStatus, HbciJobResult, HbciJobResultData,
    HbciReturnValue, Konto, Limit, Saldo, Value,
};
pub use manager::{
    BankInfo, BankInfoRegistry, HbciHandler, HbciVersion, done, get_param, init, set_param,
};
pub use passport::{PassportStorage, PinTanPassport, PinTanPassportData};
