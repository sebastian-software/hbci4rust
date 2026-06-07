use std::env;
use std::sync::Arc;

use async_trait::async_trait;
use hbci4rust::{
    CallbackEvent, CallbackReason, CallbackResponse, HbciCallback, HbciError, HbciErrorKind,
    HbciHandler, HbciResult, PinTanPassport, PinTanPassportData, done, init,
};

#[derive(Debug)]
struct LiveEnvCallback;

#[async_trait]
impl HbciCallback for LiveEnvCallback {
    async fn handle(&self, event: CallbackEvent) -> HbciResult<CallbackResponse> {
        match event.reason {
            CallbackReason::NeedCountry => Ok(CallbackResponse::value(env_or(
                "HBCI4RUST_LIVE_COUNTRY",
                "DE",
            ))),
            CallbackReason::NeedBlz => {
                Ok(CallbackResponse::value(env_required("HBCI4RUST_LIVE_BLZ")?))
            }
            CallbackReason::NeedHost => Ok(CallbackResponse::value(env_required(
                "HBCI4RUST_LIVE_HOST",
            )?)),
            CallbackReason::NeedFilter => Ok(env_optional("HBCI4RUST_LIVE_FILTER")
                .map(CallbackResponse::value)
                .unwrap_or_else(CallbackResponse::empty)),
            CallbackReason::NeedUserId => Ok(CallbackResponse::value(env_required(
                "HBCI4RUST_LIVE_USER_ID",
            )?)),
            CallbackReason::NeedCustomerId => Ok(env_optional("HBCI4RUST_LIVE_CUSTOMER_ID")
                .map(CallbackResponse::value)
                .unwrap_or_else(CallbackResponse::empty)),
            CallbackReason::NeedPtPin => {
                Ok(CallbackResponse::value(env_required("HBCI4RUST_LIVE_PIN")?))
            }
            CallbackReason::NeedPtTan => {
                Ok(CallbackResponse::value(env_required("HBCI4RUST_LIVE_TAN")?))
            }
            CallbackReason::NeedPtSecMech => Ok(env_optional("HBCI4RUST_LIVE_TAN_METHOD")
                .map(CallbackResponse::value)
                .unwrap_or_else(CallbackResponse::empty)),
            CallbackReason::NeedPtTanMedia => Ok(env_optional("HBCI4RUST_LIVE_TAN_MEDIA")
                .map(CallbackResponse::value)
                .unwrap_or_else(CallbackResponse::empty)),
            CallbackReason::NeedConnection
            | CallbackReason::CloseConnection
            | CallbackReason::HaveInstMsg
            | CallbackReason::HaveCrcError
            | CallbackReason::HaveError
            | CallbackReason::HaveIbanError
            | CallbackReason::NeedPort
            | CallbackReason::Unknown(_) => Ok(CallbackResponse::empty()),
        }
    }
}

#[tokio::test]
#[ignore = "requires explicit HBCI4RUST_LIVE_ENABLE=1 and real PinTAN credentials"]
async fn live_pintan_dialog_init_and_close_from_env() -> HbciResult<()> {
    if env::var("HBCI4RUST_LIVE_ENABLE").as_deref() != Ok("1") {
        eprintln!("skipping live bank smoke test; set HBCI4RUST_LIVE_ENABLE=1 to run");
        return Ok(());
    }

    init(
        std::iter::empty::<(&str, &str)>(),
        Arc::new(LiveEnvCallback),
    )?;
    let result = run_live_dialog_init_and_close().await;
    let cleanup = done();

    cleanup?;
    result
}

async fn run_live_dialog_init_and_close() -> HbciResult<()> {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: env_or("HBCI4RUST_LIVE_COUNTRY", "DE"),
        blz: env_required("HBCI4RUST_LIVE_BLZ")?,
        host: Some(env_required("HBCI4RUST_LIVE_HOST")?),
        user_id: env_required("HBCI4RUST_LIVE_USER_ID")?,
        customer_id: env_optional("HBCI4RUST_LIVE_CUSTOMER_ID"),
        filter: env_optional("HBCI4RUST_LIVE_FILTER"),
        tan_method: env_optional("HBCI4RUST_LIVE_TAN_METHOD"),
        tan_media: env_optional("HBCI4RUST_LIVE_TAN_MEDIA"),
        ..PinTanPassportData::default()
    });
    let mut handler = HbciHandler::new(env_or("HBCI4RUST_LIVE_HBCI_VERSION", "300"), passport);

    handler.init().await?;
    handler.close().await
}

fn env_required(name: &str) -> HbciResult<String> {
    env_optional(name).ok_or_else(|| {
        HbciError::new(
            HbciErrorKind::InvalidArgument,
            format!("missing required live test env var {name}"),
        )
    })
}

fn env_optional(name: &str) -> Option<String> {
    env::var(name)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn env_or(name: &str, default: &str) -> String {
    env_optional(name).unwrap_or_else(|| default.to_owned())
}
