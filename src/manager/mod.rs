mod account_crc;
mod bank_info;
mod handler;
mod orderhash;
mod secmech;
mod signature;

use std::collections::BTreeMap;
use std::sync::{Arc, OnceLock, RwLock};

use crate::callback::HbciCallback;
use crate::error::{HbciError, HbciErrorKind, HbciResult};

pub use account_crc::AccountCrcAlgs;
pub use bank_info::{BankInfo, BankInfoRegistry, HbciVersion};
pub use handler::HbciHandler;
pub use orderhash::OrderHashMode;
pub use secmech::{
    AppliedChallengeParams, ChallengeHhdVersion, ChallengeInfo, ChallengeJob, ChallengeParam,
    FlickerCode, FlickerCodeVersion, FlickerDataElement, FlickerEncoding, FlickerRenderer,
    FlickerStartCode, HhdVersion, HhdVersionType, MatrixCode, QrCode,
};
pub use signature::{PinTanSigHead, apply_pintan_sig_head, apply_pintan_user_sig_to_sig_tail};

#[derive(Default)]
struct RuntimeState {
    params: BTreeMap<String, String>,
    callback: Option<Arc<dyn HbciCallback>>,
}

static RUNTIME: OnceLock<RwLock<RuntimeState>> = OnceLock::new();

fn runtime() -> &'static RwLock<RuntimeState> {
    RUNTIME.get_or_init(|| RwLock::new(RuntimeState::default()))
}

pub fn init<K, V, I>(params: I, callback: Arc<dyn HbciCallback>) -> HbciResult<()>
where
    K: Into<String>,
    V: Into<String>,
    I: IntoIterator<Item = (K, V)>,
{
    let mut state = runtime()
        .write()
        .map_err(|_| HbciError::new(HbciErrorKind::Config, "runtime lock poisoned"))?;
    state.params = params
        .into_iter()
        .map(|(key, value)| (key.into(), value.into()))
        .collect();
    state.callback = Some(callback);
    Ok(())
}

pub fn done() -> HbciResult<()> {
    let mut state = runtime()
        .write()
        .map_err(|_| HbciError::new(HbciErrorKind::Config, "runtime lock poisoned"))?;
    state.params.clear();
    state.callback = None;
    Ok(())
}

pub fn set_param(name: impl Into<String>, value: impl Into<String>) -> HbciResult<()> {
    runtime()
        .write()
        .map_err(|_| HbciError::new(HbciErrorKind::Config, "runtime lock poisoned"))?
        .params
        .insert(name.into(), value.into());
    Ok(())
}

pub fn get_param(name: &str) -> Option<String> {
    runtime()
        .read()
        .ok()
        .and_then(|state| state.params.get(name).cloned())
}

pub(crate) fn callback() -> Option<Arc<dyn HbciCallback>> {
    runtime()
        .read()
        .ok()
        .and_then(|state| state.callback.as_ref().cloned())
}
