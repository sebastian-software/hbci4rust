use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use hbci4rust::{
    CallbackDataType, CallbackEvent, CallbackReason, CallbackResponse, CommResponse, HbciCallback,
    HbciHandler, HbciJobResultData, HbciResult, Konto, Limit, PassportStorage, PinTanPassport,
    PinTanPassportData, ReplayCommClient, Value, done, init,
    protocol::{load_protocol_spec, parse_wire_message},
};

static RUNTIME_CALLBACK_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Debug)]
struct RecordingCallback {
    events: Arc<Mutex<Vec<CallbackEvent>>>,
}

#[async_trait]
impl HbciCallback for RecordingCallback {
    async fn handle(&self, event: CallbackEvent) -> HbciResult<CallbackResponse> {
        self.events.lock().expect("callback event lock").push(event);
        Ok(CallbackResponse::empty())
    }
}

#[derive(Debug)]
struct ScriptedCallback {
    events: Arc<Mutex<Vec<CallbackEvent>>>,
    responses: Arc<Mutex<VecDeque<CallbackResponse>>>,
}

#[async_trait]
impl HbciCallback for ScriptedCallback {
    async fn handle(&self, event: CallbackEvent) -> HbciResult<CallbackResponse> {
        self.events.lock().expect("callback event lock").push(event);
        Ok(self
            .responses
            .lock()
            .expect("callback response lock")
            .pop_front()
            .unwrap_or_else(CallbackResponse::empty))
    }
}

fn custom_msg_response(body_segments: &[&str]) -> CommResponse {
    custom_msg_response_for_request("0", 1, body_segments)
}

fn custom_msg_response_for_request(
    ref_dialog_id: &str,
    ref_msgnum: u32,
    body_segments: &[&str],
) -> CommResponse {
    fints_response("DIALOG1", 1, ref_dialog_id, ref_msgnum, body_segments)
}

fn fints_response(
    dialog_id: &str,
    msgnum: u32,
    ref_dialog_id: &str,
    ref_msgnum: u32,
    body_segments: &[&str],
) -> CommResponse {
    let mut body =
        format!("HNHBK:1:3+000000000123+300+{dialog_id}+{msgnum}+{ref_dialog_id}:{ref_msgnum}'");
    for segment in body_segments {
        body.push_str(segment);
        body.push('\'');
    }
    body.push_str("HNHBS:");
    body.push_str(&(body_segments.len() + 2).to_string());
    body.push_str(":1+");
    body.push_str(&msgnum.to_string());
    body.push('\'');
    CommResponse::ok(body)
}

fn custom_msg_ok_response() -> CommResponse {
    custom_msg_response(&["HIRMG:2:2+0010::OK"])
}

fn giro_account() -> Konto {
    Konto {
        country: Some("DE".to_owned()),
        blz: Some("12345678".to_owned()),
        number: Some("0001234567".to_owned()),
        subnumber: None,
        bic: Some("MARKDEF1100".to_owned()),
        iban: Some("DE02123456780000000000".to_owned()),
        customer_id: Some("customer".to_owned()),
        name: Some("Max Mustermann".to_owned()),
        name2: None,
        acctype: Some("1".to_owned()),
        account_type: Some("Girokonto".to_owned()),
        curr: Some("EUR".to_owned()),
        limit: Some(Limit {
            limit_type: Limit::TYPE_DAILY.to_owned(),
            value: Some(Value {
                value: "1000.00".to_owned(),
                curr: Some("EUR".to_owned()),
            }),
            days: None,
        }),
        allowed_gvs: vec!["HKSAL".to_owned(), "HKWPD".to_owned()],
    }
}

#[test]
fn creates_java_named_job_with_string_params() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);

    let mut job = handler.new_job("SaldoReq").expect("job is in registry");
    job.set_param("src.iban", "DE1234567890");

    assert_eq!(job.name(), "SaldoReq");
    assert_eq!(job.param("src.iban"), Some("DE1234567890"));
    assert!(job.lowlevel_params().is_empty());
}

#[test]
fn saldo_jobs_expose_original_near_constraints() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);

    let saldo = handler.new_job("SaldoReq").expect("job is in registry");

    assert_eq!(saldo.constraints().len(), 8);
    let iban = saldo.constraint("my.iban").expect("iban constraint");
    assert_eq!(iban.destination_name, "Saldo7.KTV.iban");
    assert_eq!(iban.default_value, None);
    let country = saldo.constraint("my.country").expect("country constraint");
    assert_eq!(country.destination_name, "Saldo7.KTV.KIK.country");
    assert_eq!(country.default_value.as_deref(), Some("DE"));
    let dummyall = saldo.constraint("dummyall").expect("dummyall constraint");
    assert_eq!(dummyall.destination_name, "Saldo7.allaccounts");
    assert_eq!(dummyall.default_value.as_deref(), Some("N"));

    let saldo_all = handler.new_job("SaldoReqAll").expect("job is in registry");
    let dummyall_all = saldo_all
        .constraint("dummyall")
        .expect("SaldoReqAll dummyall constraint");
    let iban_all = saldo_all
        .constraint("my.iban")
        .expect("SaldoReqAll iban constraint");

    assert_eq!(saldo_all.constraints().len(), 8);
    assert_eq!(dummyall_all.default_value.as_deref(), Some("J"));
    assert_eq!(iban_all.destination_name, "Saldo7.KTV.iban");
    assert_eq!(saldo_all.constraint("my.curr"), None);
}

#[test]
fn checked_job_param_setter_accepts_known_non_empty_param() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");

    saldo
        .try_set_param("my.iban", "DE02123456780000000000")
        .expect("known parameter is accepted");

    assert_eq!(saldo.param("my.iban"), Some("DE02123456780000000000"));
    assert_eq!(
        saldo.lowlevel_param("Saldo7.KTV.iban"),
        Some("DE02123456780000000000")
    );
}

#[test]
fn checked_job_param_setter_rejects_unaccepted_param_like_original() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");

    let err = saldo
        .try_set_param("src.iban", "DE02123456780000000000")
        .expect_err("unknown high-level parameter is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(
        err.message(),
        "job parameter src.iban is not accepted by SaldoReq"
    );
    assert!(saldo.params().is_empty());
}

#[test]
fn checked_job_param_setter_rejects_empty_value_like_original() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");

    let err = saldo
        .try_set_param("my.iban", "")
        .expect_err("empty high-level parameter value is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(
        err.message(),
        "job parameter my.iban must not be empty for SaldoReq"
    );
    assert!(saldo.params().is_empty());
}

#[test]
fn integer_param_setter_wraps_permissive_string_param() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");

    saldo.set_param_int("custom.count", -7);

    assert_eq!(saldo.param("custom.count"), Some("-7"));
    assert!(saldo.lowlevel_params().is_empty());
}

#[test]
fn checked_integer_param_setter_uses_original_string_shape() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");

    saldo
        .try_set_param_int("maxentries", 25)
        .expect("integer parameter is accepted");

    assert_eq!(saldo.param("maxentries"), Some("25"));
    assert_eq!(saldo.lowlevel_param("Saldo7.maxentries"), Some("25"));
}

#[test]
fn checked_integer_param_setter_rejects_unaccepted_param_like_original() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");

    let err = saldo
        .try_set_param_int("custom.count", 25)
        .expect_err("unknown integer parameter is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(
        err.message(),
        "job parameter custom.count is not accepted by SaldoReq"
    );
    assert!(saldo.params().is_empty());
}

#[test]
fn indexed_job_param_setter_inserts_index_like_original() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "IndexedJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "line.value",
                    "destination_name": "IndexedSeg.lines.entry.value",
                    "default_value": null,
                    "indexed": true
                }
            ]
        }"#,
    )
    .expect("indexed job");

    job.try_set_indexed_param("line.value", 3, "hello")
        .expect("indexed parameter is accepted");

    assert_eq!(
        job.lowlevel_param("IndexedSeg.lines.entry[3].value"),
        Some("hello")
    );
    assert_eq!(job.param("line.value"), None);
}

#[test]
fn indexed_job_param_setter_rejects_non_indexed_param_like_original() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");

    let err = saldo
        .try_set_indexed_param("my.iban", 0, "DE02123456780000000000")
        .expect_err("non-indexed high-level parameter is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(
        err.message(),
        "job parameter my.iban is not indexed by SaldoReq"
    );
    assert!(saldo.lowlevel_params().is_empty());
}

#[test]
fn indexed_value_param_helper_sets_amount_and_currency_like_original() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "IndexedValueJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "btg.value",
                    "destination_name": "ValueSeg.lines.BTG.value",
                    "default_value": null,
                    "indexed": true
                },
                {
                    "frontend_name": "btg.curr",
                    "destination_name": "ValueSeg.lines.BTG.curr",
                    "default_value": null,
                    "indexed": true
                }
            ]
        }"#,
    )
    .expect("indexed value job");

    job.try_set_indexed_param_value(
        "btg",
        2,
        &Value {
            value: "123.45".to_owned(),
            curr: Some("EUR".to_owned()),
        },
    )
    .expect("indexed value parameter is accepted");

    assert_eq!(job.param("btg.value"), None);
    assert_eq!(
        job.lowlevel_param("ValueSeg.lines.BTG[2].value"),
        Some("123.45")
    );
    assert_eq!(
        job.lowlevel_param("ValueSeg.lines.BTG[2].curr"),
        Some("EUR")
    );
}

#[test]
fn indexed_value_param_helper_ignores_unaccepted_or_empty_fields_like_original() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "IndexedValueJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "btg.value",
                    "destination_name": "ValueSeg.lines.BTG.value",
                    "default_value": null,
                    "indexed": true
                }
            ]
        }"#,
    )
    .expect("indexed value job");

    job.try_set_indexed_param_value(
        "btg",
        1,
        &Value {
            value: "50.00".to_owned(),
            curr: Some(String::new()),
        },
    )
    .expect("indexed value with empty currency is accepted");
    job.try_set_indexed_param_value(
        "fee",
        1,
        &Value {
            value: "1.00".to_owned(),
            curr: Some("EUR".to_owned()),
        },
    )
    .expect("unaccepted indexed value fields are ignored");

    assert_eq!(
        job.lowlevel_param("ValueSeg.lines.BTG[1].value"),
        Some("50.00")
    );
    assert_eq!(job.lowlevel_param("ValueSeg.lines.BTG[1].curr"), None);
    assert!(job.params().is_empty());
}

#[test]
fn indexed_value_param_helper_rejects_accepted_non_indexed_field() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "IndexedValueJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "btg.value",
                    "destination_name": "ValueSeg.BTG.value",
                    "default_value": null,
                    "indexed": false
                }
            ]
        }"#,
    )
    .expect("indexed value job");

    let err = job
        .try_set_indexed_param_value(
            "btg",
            0,
            &Value {
                value: "123.45".to_owned(),
                curr: Some("EUR".to_owned()),
            },
        )
        .expect_err("accepted non-indexed value field is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(
        err.message(),
        "job parameter btg.value is not indexed by IndexedValueJob"
    );
    assert!(job.lowlevel_params().is_empty());
}

#[test]
fn indexed_account_param_helper_sets_account_fields_like_original() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "IndexedAccountJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "acct.country",
                    "destination_name": "AccountSeg.lines.KTV.country",
                    "default_value": null,
                    "indexed": true
                },
                {
                    "frontend_name": "acct.blz",
                    "destination_name": "AccountSeg.lines.KTV.blz",
                    "default_value": null,
                    "indexed": true
                },
                {
                    "frontend_name": "acct.number",
                    "destination_name": "AccountSeg.lines.KTV.number",
                    "default_value": null,
                    "indexed": true
                },
                {
                    "frontend_name": "acct.iban",
                    "destination_name": "AccountSeg.lines.KTV.iban",
                    "default_value": null,
                    "indexed": true
                }
            ]
        }"#,
    )
    .expect("indexed account job");

    job.try_set_indexed_param_account("acct", 4, &giro_account())
        .expect("indexed account parameter is accepted");

    assert_eq!(job.param("acct.iban"), None);
    assert_eq!(
        job.lowlevel_param("AccountSeg.lines.KTV[4].country"),
        Some("DE")
    );
    assert_eq!(
        job.lowlevel_param("AccountSeg.lines.KTV[4].blz"),
        Some("12345678")
    );
    assert_eq!(
        job.lowlevel_param("AccountSeg.lines.KTV[4].number"),
        Some("0001234567")
    );
    assert_eq!(
        job.lowlevel_param("AccountSeg.lines.KTV[4].iban"),
        Some("DE02123456780000000000")
    );
}

#[test]
fn indexed_account_param_helper_ignores_unaccepted_or_empty_fields_like_original() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "IndexedAccountJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "acct.number",
                    "destination_name": "AccountSeg.lines.KTV.number",
                    "default_value": null,
                    "indexed": true
                }
            ]
        }"#,
    )
    .expect("indexed account job");
    let mut account = giro_account();
    account.iban = Some(String::new());

    job.try_set_indexed_param_account("acct", 1, &account)
        .expect("indexed account with unaccepted or empty fields is accepted");
    job.try_set_indexed_param_account("other", 1, &giro_account())
        .expect("unaccepted indexed account fields are ignored");

    assert_eq!(
        job.lowlevel_param("AccountSeg.lines.KTV[1].number"),
        Some("0001234567")
    );
    assert_eq!(job.lowlevel_param("AccountSeg.lines.KTV[1].iban"), None);
    assert!(job.params().is_empty());
}

#[test]
fn indexed_account_param_helper_rejects_accepted_non_indexed_field() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "IndexedAccountJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "acct.iban",
                    "destination_name": "AccountSeg.KTV.iban",
                    "default_value": null,
                    "indexed": false
                }
            ]
        }"#,
    )
    .expect("indexed account job");

    let err = job
        .try_set_indexed_param_account("acct", 0, &giro_account())
        .expect_err("accepted non-indexed account field is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(
        err.message(),
        "job parameter acct.iban is not indexed by IndexedAccountJob"
    );
    assert!(job.lowlevel_params().is_empty());
}

#[test]
fn saldo_job_sets_account_params_like_original_overload() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let account = giro_account();
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");

    saldo.set_param_account("my", &account);

    assert_eq!(saldo.param("my.country"), Some("DE"));
    assert_eq!(saldo.param("my.blz"), Some("12345678"));
    assert_eq!(saldo.param("my.number"), Some("0001234567"));
    assert_eq!(saldo.param("my.bic"), Some("MARKDEF1100"));
    assert_eq!(saldo.param("my.iban"), Some("DE02123456780000000000"));
    assert_eq!(saldo.param("my.name"), None);
    assert_eq!(saldo.param("my.curr"), None);
    assert_eq!(saldo.lowlevel_param("Saldo7.KTV.KIK.country"), Some("DE"));
    assert_eq!(saldo.lowlevel_param("Saldo7.KTV.KIK.blz"), Some("12345678"));
    assert_eq!(
        saldo.lowlevel_param("Saldo7.KTV.number"),
        Some("0001234567")
    );
    assert_eq!(saldo.lowlevel_param("Saldo7.KTV.bic"), Some("MARKDEF1100"));
    assert_eq!(
        saldo.lowlevel_param("Saldo7.KTV.iban"),
        Some("DE02123456780000000000")
    );
    assert_eq!(saldo.lowlevel_param("Saldo7.curr"), None);
}

#[test]
fn account_param_helper_ignores_unaccepted_or_empty_fields_like_original() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut account = giro_account();
    account.iban = Some(String::new());
    account.subnumber = Some(String::new());
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    let mut saldo_all = handler.new_job("SaldoReqAll").expect("job is in registry");

    saldo.set_param_account("my", &account);
    saldo_all.set_param_account("my", &account);

    assert_eq!(saldo.param("my.iban"), None);
    assert_eq!(saldo.param("my.subnumber"), None);
    assert_eq!(saldo.param("my.number"), Some("0001234567"));
    assert_eq!(saldo_all.param("my.iban"), None);
    assert_eq!(saldo_all.param("my.subnumber"), None);
    assert_eq!(saldo_all.param("my.number"), Some("0001234567"));
    assert_eq!(saldo_all.param("my.curr"), None);
    assert_eq!(
        saldo_all.lowlevel_param("Saldo7.KTV.number"),
        Some("0001234567")
    );
}

#[test]
fn value_param_helper_sets_amount_and_currency_like_original_overload() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "ValueJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "btg.value",
                    "destination_name": "ValueSeg.BTG.value",
                    "default_value": null,
                    "indexed": false
                },
                {
                    "frontend_name": "btg.curr",
                    "destination_name": "ValueSeg.BTG.curr",
                    "default_value": null,
                    "indexed": false
                }
            ]
        }"#,
    )
    .expect("value job");

    job.set_param_value(
        "btg",
        &Value {
            value: "123.45".to_owned(),
            curr: Some("EUR".to_owned()),
        },
    );

    assert_eq!(job.param("btg.value"), Some("123.45"));
    assert_eq!(job.param("btg.curr"), Some("EUR"));
    assert_eq!(job.lowlevel_param("ValueSeg.BTG.value"), Some("123.45"));
    assert_eq!(job.lowlevel_param("ValueSeg.BTG.curr"), Some("EUR"));
}

#[test]
fn value_param_helper_ignores_unaccepted_or_empty_fields_like_original() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "ValueJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "btg.value",
                    "destination_name": "ValueSeg.BTG.value",
                    "default_value": null,
                    "indexed": false
                }
            ]
        }"#,
    )
    .expect("value job");

    job.set_param_value(
        "btg",
        &Value {
            value: "50.00".to_owned(),
            curr: Some(String::new()),
        },
    );
    job.set_param_value(
        "fee",
        &Value {
            value: "1.00".to_owned(),
            curr: Some("EUR".to_owned()),
        },
    );

    assert_eq!(job.param("btg.value"), Some("50.00"));
    assert_eq!(job.param("btg.curr"), None);
    assert_eq!(job.param("fee.value"), None);
    assert_eq!(job.lowlevel_param("ValueSeg.BTG.value"), Some("50.00"));
}

#[test]
fn verify_constraints_resolves_frontend_params_and_defaults_like_original() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut account = giro_account();
    account.country = None;
    account.subnumber = None;
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");

    saldo.set_param_account("my", &account);
    let lowlevel = saldo.verify_constraints().expect("constraints resolve");

    assert_eq!(
        lowlevel.get("Saldo7.KTV.iban").map(String::as_str),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        lowlevel.get("Saldo7.KTV.bic").map(String::as_str),
        Some("MARKDEF1100")
    );
    assert_eq!(
        lowlevel.get("Saldo7.KTV.KIK.country").map(String::as_str),
        Some("DE")
    );
    assert_eq!(
        lowlevel.get("Saldo7.KTV.KIK.blz").map(String::as_str),
        Some("12345678")
    );
    assert_eq!(
        lowlevel.get("Saldo7.KTV.number").map(String::as_str),
        Some("0001234567")
    );
    assert_eq!(
        lowlevel.get("Saldo7.allaccounts").map(String::as_str),
        Some("N")
    );
    assert_eq!(saldo.lowlevel_param("Saldo7.KTV.KIK.country"), Some("DE"));
    assert_eq!(saldo.lowlevel_param("Saldo7.allaccounts"), Some("N"));
    assert!(!lowlevel.contains_key("Saldo7.KTV.subnumber"));
    assert!(!lowlevel.contains_key("Saldo7.maxentries"));
}

#[test]
fn verify_constraints_resolves_existing_lowlevel_params_like_original() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "SaldoReq",
            "params": {},
            "lowlevel_params": {
                "Saldo7.KTV.iban": "DE02123456780000000000"
            },
            "constraints": [
                {
                    "frontend_name": "my.iban",
                    "destination_name": "Saldo7.KTV.iban",
                    "default_value": null,
                    "indexed": false
                },
                {
                    "frontend_name": "dummyall",
                    "destination_name": "Saldo7.allaccounts",
                    "default_value": "N",
                    "indexed": false
                }
            ]
        }"#,
    )
    .expect("job with lowlevel params");

    let lowlevel = job.verify_constraints().expect("constraints resolve");

    assert_eq!(
        lowlevel.get("Saldo7.KTV.iban").map(String::as_str),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        lowlevel.get("Saldo7.allaccounts").map(String::as_str),
        Some("N")
    );
    assert_eq!(job.lowlevel_param("Saldo7.allaccounts"), Some("N"));
}

#[test]
fn verify_constraints_prefers_lowlevel_over_frontend_like_original() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "SaldoReq",
            "params": {
                "my.iban": "FRONTEND"
            },
            "lowlevel_params": {
                "Saldo7.KTV.iban": "LOWLEVEL"
            },
            "constraints": [
                {
                    "frontend_name": "my.iban",
                    "destination_name": "Saldo7.KTV.iban",
                    "default_value": null,
                    "indexed": false
                }
            ]
        }"#,
    )
    .expect("job with frontend and lowlevel params");

    let lowlevel = job.verify_constraints().expect("constraints resolve");

    assert_eq!(
        lowlevel.get("Saldo7.KTV.iban").map(String::as_str),
        Some("LOWLEVEL")
    );
}

#[test]
fn verify_constraints_resolves_indexed_zero_lowlevel_like_original() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "IndexedValueJob",
            "params": {},
            "lowlevel_params": {
                "ValueSeg.lines.BTG[0].value": "123.45"
            },
            "constraints": [
                {
                    "frontend_name": "btg.value",
                    "destination_name": "ValueSeg.lines.BTG.value",
                    "default_value": null,
                    "indexed": true
                }
            ]
        }"#,
    )
    .expect("indexed job with lowlevel params");

    let lowlevel = job.verify_constraints().expect("constraints resolve");

    assert_eq!(
        lowlevel.get("ValueSeg.lines.BTG.value").map(String::as_str),
        Some("123.45")
    );
    assert_eq!(
        job.lowlevel_param("ValueSeg.lines.BTG[0].value"),
        Some("123.45")
    );
    assert_eq!(job.lowlevel_param("ValueSeg.lines.BTG.value"), None);
}

#[test]
fn verify_constraints_prefers_unindexed_lowlevel_over_indexed_zero_like_original() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "IndexedValueJob",
            "params": {},
            "lowlevel_params": {
                "ValueSeg.lines.BTG.value": "UNINDEXED",
                "ValueSeg.lines.BTG[0].value": "INDEXED"
            },
            "constraints": [
                {
                    "frontend_name": "btg.value",
                    "destination_name": "ValueSeg.lines.BTG.value",
                    "default_value": null,
                    "indexed": true
                }
            ]
        }"#,
    )
    .expect("indexed job with lowlevel params");

    let lowlevel = job.verify_constraints().expect("constraints resolve");

    assert_eq!(
        lowlevel.get("ValueSeg.lines.BTG.value").map(String::as_str),
        Some("UNINDEXED")
    );
}

#[test]
fn verify_constraints_persists_indexed_defaults_unindexed_like_original() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "IndexedDefaultJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "line.mode",
                    "destination_name": "LineSeg.lines.mode",
                    "default_value": "N",
                    "indexed": true
                }
            ]
        }"#,
    )
    .expect("indexed job with default constraint");

    let lowlevel = job.verify_constraints().expect("constraints resolve");

    assert_eq!(
        lowlevel.get("LineSeg.lines.mode").map(String::as_str),
        Some("N")
    );
    assert_eq!(job.lowlevel_param("LineSeg.lines.mode"), Some("N"));
    assert_eq!(job.lowlevel_param("LineSeg.lines.mode[0]"), None);
}

#[test]
fn verify_constraints_reports_missing_required_frontend_param() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param("my.bic", "MARKDEF1100");

    let err = saldo
        .verify_constraints()
        .expect_err("missing iban is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(err.message(), "missing required job parameter: my.iban");
}

#[test]
fn verify_constraints_for_saldo_all_resolves_account_and_all_default() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut saldo_all = handler.new_job("SaldoReqAll").expect("job is in registry");
    saldo_all.set_param_account("my", &giro_account());

    let lowlevel = saldo_all
        .verify_constraints()
        .expect("SaldoReqAll defaults resolve");

    assert_eq!(
        lowlevel.get("Saldo7.KTV.iban").map(String::as_str),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        lowlevel.get("Saldo7.allaccounts").map(String::as_str),
        Some("J")
    );
    assert_eq!(
        saldo_all.lowlevel_param("Saldo7.KTV.iban"),
        Some("DE02123456780000000000")
    );
    assert_eq!(saldo_all.lowlevel_param("Saldo7.allaccounts"), Some("J"));
    assert!(!lowlevel.contains_key("Saldo7.maxentries"));
}

#[test]
fn checked_queue_add_verifies_constraints_like_original_add_task() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let mut handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &giro_account());

    handler
        .try_add_to_queue(saldo)
        .expect("verified job is queued");

    let queued = &handler.queued_jobs()[0];
    assert_eq!(queued.lowlevel_param("Saldo7.allaccounts"), Some("N"));
    assert_eq!(
        queued.lowlevel_param("Saldo7.KTV.iban"),
        Some("DE02123456780000000000")
    );
    assert_eq!(queued.lowlevel_param("Saldo7.maxentries"), None);
}

#[tokio::test]
async fn async_checked_queue_add_corrects_invalid_iban_through_global_callback() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    let responses = Arc::new(Mutex::new(VecDeque::from([CallbackResponse::value(
        "DE89370400440532013000",
    )])));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(ScriptedCallback {
            events: events.clone(),
            responses,
        }),
    )
    .expect("runtime init");

    let passport = PinTanPassport::new(PinTanPassportData::default());
    let mut handler = HbciHandler::new("300", passport);
    let mut account = giro_account();
    account.blz = Some("37040044".to_owned());
    account.number = Some("0532013000".to_owned());
    account.bic = Some("COBADEFFXXX".to_owned());
    account.iban = Some("DE89370400440532013001".to_owned());
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &account);

    handler
        .try_add_to_queue_with_account_checks(saldo)
        .await
        .expect("account-checked job is queued");

    let queued = &handler.queued_jobs()[0];
    assert_eq!(
        queued.lowlevel_param("Saldo7.KTV.iban"),
        Some("DE89370400440532013000")
    );
    assert_eq!(queued.param("my.iban"), Some("DE89370400440532013000"));

    let events = events.lock().expect("callback event lock");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].reason, CallbackReason::HaveIbanError);
    assert_eq!(events[0].data_type, CallbackDataType::Text);
    assert_eq!(
        events[0].current_value.as_deref(),
        Some("DE89370400440532013001")
    );
    drop(events);
    done().expect("runtime reset");
}

#[test]
fn checked_queue_add_rejects_missing_required_job_data() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let mut handler = HbciHandler::new("300", passport);
    let saldo = handler.new_job("SaldoReq").expect("job is in registry");

    let err = handler
        .try_add_to_queue(saldo)
        .expect_err("missing required SaldoReq data is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(err.message(), "missing required job parameter: my.bic");
    assert!(handler.queued_jobs().is_empty());
}

#[test]
fn rejects_out_of_scope_job() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);

    assert!(handler.new_job("ChipcardOnly").is_err());
}

#[test]
fn konto_reports_sepa_account_when_bic_and_iban_are_present() {
    assert!(giro_account().is_sepa_account());

    let mut missing_bic = giro_account();
    missing_bic.bic = None;
    assert!(!missing_bic.is_sepa_account());

    let mut empty_iban = giro_account();
    empty_iban.iban = Some(String::new());
    assert!(!empty_iban.is_sepa_account());
}

#[test]
fn konto_checks_iban_crc_with_original_uppercase_input_boundary() {
    let mut account = giro_account();
    account.iban = Some("DE89370400440532013000".to_owned());
    assert!(account.check_iban());

    account.iban = Some("DE89370400440532013001".to_owned());
    assert!(!account.check_iban());

    account.iban = Some("de89370400440532013000".to_owned());
    assert!(!account.check_iban());

    account.iban = None;
    assert!(!account.check_iban());
}

#[test]
fn konto_default_matches_original_constructor_currency() {
    let account = Konto::default();

    assert_eq!(account.curr.as_deref(), Some("EUR"));
    assert_eq!(account.to_string(), " (EUR)");
}

#[test]
fn konto_equality_matches_original_compared_fields() {
    let left = giro_account();
    let mut right = giro_account();
    right.acctype = Some("9".to_owned());
    right.limit = None;
    right.allowed_gvs = vec!["HKCCS".to_owned()];

    assert_eq!(left, right);

    let mut different_iban = left.clone();
    different_iban.iban = Some("DE89370400440532013000".to_owned());
    assert_ne!(left, different_iban);
}

#[test]
fn konto_display_matches_original_field_order_without_bank_info() {
    assert_eq!(
        giro_account().to_string(),
        "Girokonto Max Mustermann 0001234567 BLZ 12345678 () BIC MARKDEF1100 IBAN DE02123456780000000000 [DE] (EUR)"
    );
}

#[test]
fn passport_account_by_number_returns_cached_account() {
    let passport = PinTanPassport::new(PinTanPassportData {
        accounts: vec![giro_account()],
        ..PinTanPassportData::default()
    });

    let account = passport.account_by_number("1234567");

    assert_eq!(account.number.as_deref(), Some("0001234567"));
    assert_eq!(account.blz.as_deref(), Some("12345678"));
    assert_eq!(account.country.as_deref(), Some("DE"));
    assert_eq!(account.customer_id.as_deref(), Some("customer"));
    assert_eq!(account.curr.as_deref(), Some("EUR"));
    assert_eq!(account.iban.as_deref(), Some("DE02123456780000000000"));
    assert_eq!(account.bic.as_deref(), Some("MARKDEF1100"));
    assert_eq!(account.allowed_gvs, ["HKSAL", "HKWPD"]);
    assert_eq!(
        account
            .limit
            .as_ref()
            .map(|limit| limit.limit_type.as_str()),
        Some(Limit::TYPE_DAILY)
    );
}

#[test]
fn passport_account_by_number_falls_back_to_passport_identity() {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    });

    let account = passport.account_by_number("9999999999");

    assert_eq!(account.number.as_deref(), Some("9999999999"));
    assert_eq!(account.blz.as_deref(), Some("12345678"));
    assert_eq!(account.country.as_deref(), Some("DE"));
    assert_eq!(account.customer_id.as_deref(), Some("customer"));
    assert_eq!(account.name.as_deref(), Some("customer"));
    assert_eq!(account.curr.as_deref(), Some("EUR"));
}

#[test]
fn rust_native_passport_storage_roundtrips() {
    let data = PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        filter: Some("Base64".to_owned()),
        tan_method: Some("921".to_owned()),
        tan_media: Some("phone".to_owned()),
        bpd_version: Some("5".to_owned()),
        upd_version: Some("7".to_owned()),
        bank_name: Some("Test Bank".to_owned()),
        max_gv_per_message: Some(4),
        max_message_size_kb: Some(2048),
        supported_languages: vec!["1".to_owned(), "2".to_owned()],
        supported_hbci_versions: vec!["300".to_owned(), "220".to_owned()],
        upd_usage: Some("1".to_owned()),
        user_name: Some("Max Mustermann".to_owned()),
        accounts: vec![giro_account()],
    };

    let bytes = PassportStorage::save_to_vec(&data, b"correct horse battery staple")
        .expect("passport saves");
    let restored = PassportStorage::load_from_slice(&bytes, b"correct horse battery staple")
        .expect("passport loads");

    assert_eq!(restored, data);
}

#[test]
fn passport_imports_accounts_from_dialog_init_upd_values() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");
    let message = parse_wire_message(
        "HNHBK:1:3+000000000123+300+DIALOG1+1+0:1'HIRMG:2:2+0010::Initialisiert'HIUPD:3:6+0001234567::280:12345678++customer+1+EUR+Max Mustermann++Girokonto+T:1000.00:EUR+HKSAL:1+HKWPD:1'HNHBS:4:1+1'",
    )
    .expect("wire message parses");
    let values = message
        .resolve_segments(&syntax)
        .expect("wire segments resolve")
        .values_for_message(&syntax, "DialogInitRes")
        .expect("message values extract");
    let mut passport = PinTanPassport::new(PinTanPassportData {
        accounts: vec![giro_account()],
        ..PinTanPassportData::default()
    });

    let count = passport.update_accounts_from_values(&values, "DialogInitRes.UPD");

    assert_eq!(count, 1);
    assert_eq!(passport.accounts().len(), 1);
    let account = &passport.accounts()[0];
    assert_eq!(account.number.as_deref(), Some("0001234567"));
    assert_eq!(account.country.as_deref(), Some("DE"));
    assert_eq!(account.blz.as_deref(), Some("12345678"));
    assert_eq!(account.customer_id.as_deref(), Some("customer"));
    assert_eq!(account.name.as_deref(), Some("Max Mustermann"));
    assert_eq!(account.acctype.as_deref(), Some("1"));
    assert_eq!(account.account_type.as_deref(), Some("Girokonto"));
    assert_eq!(account.curr.as_deref(), Some("EUR"));
    assert_eq!(account.iban.as_deref(), Some("DE02123456780000000000"));
    assert_eq!(account.bic.as_deref(), Some("MARKDEF1100"));
    let limit = account.limit.as_ref().expect("account limit imports");
    assert_eq!(limit.limit_type, Limit::TYPE_DAILY);
    assert_eq!(
        limit.value.as_ref().expect("limit value imports"),
        &Value {
            value: "1000.00".to_owned(),
            curr: Some("EUR".to_owned()),
        }
    );
    assert_eq!(limit.days, None);
    assert_eq!(account.allowed_gvs, ["HKSAL", "HKWPD"]);
}

#[test]
fn passport_imports_bpd_and_upd_parameter_data_from_dialog_init_values() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");
    let message = parse_wire_message(
        "HNHBK:1:3+000000000123+300+DIALOG1+1+0:1'HIRMG:2:2+0010::Initialisiert'HIBPA:3:3+5+280:12345678+Bank+4+1:2+300:220+2048'HIUPA:4:4+user+7+0+Max Mustermann'HNHBS:5:1+1'",
    )
    .expect("wire message parses");
    let values = message
        .resolve_segments(&syntax)
        .expect("wire segments resolve")
        .values_for_message(&syntax, "DialogInitRes")
        .expect("message values extract");
    let mut passport = PinTanPassport::new(PinTanPassportData::default());

    let count = passport.update_parameter_data_from_values(&values, "DialogInitRes");

    assert_eq!(count, 9);
    assert_eq!(passport.bpd_version(), "5");
    assert_eq!(passport.upd_version(), "7");
    assert_eq!(passport.bank_name(), Some("Bank"));
    assert_eq!(passport.max_gv_per_message(), Some(4));
    assert_eq!(passport.max_message_size_kb(), Some(2048));
    assert_eq!(passport.supported_languages(), ["1", "2"]);
    assert_eq!(passport.supported_hbci_versions(), ["300", "220"]);
    assert_eq!(passport.upd_usage(), Some("0"));
    assert!(passport.only_bpd_gvs());
    assert_eq!(passport.user_name(), Some("Max Mustermann"));
}

#[tokio::test]
async fn handler_init_imports_upd_accounts_from_replay_response() {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::Initialisiert",
        "HIUPD:3:6+0001234567::280:12345678+DE02123456780000000000+customer+1+EUR+Max Mustermann++Girokonto+T:1000.00:EUR+HKSAL:1+HKWPD:1",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());

    handler.init().await.expect("dialog init replay response");

    assert_eq!(
        handler.dialog_context().dialog_id.as_deref(),
        Some("DIALOG1")
    );
    assert_eq!(handler.dialog_context().message_number, 2);
    assert_eq!(handler.passport().accounts().len(), 1);
    let account = &handler.passport().accounts()[0];
    assert_eq!(account.number.as_deref(), Some("0001234567"));
    assert_eq!(account.country.as_deref(), Some("DE"));
    assert_eq!(account.blz.as_deref(), Some("12345678"));
    assert_eq!(account.customer_id.as_deref(), Some("customer"));
    assert_eq!(account.name.as_deref(), Some("Max Mustermann"));
    assert_eq!(account.iban.as_deref(), Some("DE02123456780000000000"));
    let limit = account.limit.as_ref().expect("account limit imports");
    assert_eq!(limit.limit_type, Limit::TYPE_DAILY);
    assert_eq!(
        limit.value.as_ref().expect("limit value imports"),
        &Value {
            value: "1000.00".to_owned(),
            curr: Some("EUR".to_owned()),
        }
    );
    assert_eq!(account.allowed_gvs, ["HKSAL", "HKWPD"]);

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 1);

    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert!(body.starts_with("HNHBK:1:3+"));
    assert!(body.contains("HKIDN:2:2+280:12345678+customer+0+0'"));
    assert!(body.contains("HKVVB:3:3+0+0+0+hbci4rust+0.1.0'"));
    assert!(body.ends_with("HNHBS:4:1+1'"));

    let size = &body["HNHBK:1:3+".len().."HNHBK:1:3+".len() + 12];
    assert_eq!(size, format!("{:012}", body.len()));
}

#[tokio::test]
async fn handler_init_uses_cached_bpd_and_upd_versions() {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        bpd_version: Some("5".to_owned()),
        upd_version: Some("7".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::Initialisiert",
        "HIBPA:3:3+6+280:12345678+Bank+1+1+300",
        "HIUPA:4:4+user+8+1+Max Mustermann",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());

    handler.init().await.expect("dialog init replay response");

    assert_eq!(handler.passport().bpd_version(), "6");
    assert_eq!(handler.passport().upd_version(), "8");
    assert_eq!(handler.passport().bank_name(), Some("Bank"));
    assert_eq!(handler.passport().max_gv_per_message(), Some(1));
    assert_eq!(handler.passport().supported_languages(), ["1"]);
    assert_eq!(handler.passport().supported_hbci_versions(), ["300"]);
    assert_eq!(handler.passport().upd_usage(), Some("1"));
    assert!(!handler.passport().only_bpd_gvs());
    assert_eq!(handler.passport().user_name(), Some("Max Mustermann"));

    let requests = replay.requests().expect("requests");
    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert!(body.contains("HKVVB:3:3+5+7+0+hbci4rust+0.1.0'"));
}

#[tokio::test]
async fn handler_init_emits_institute_message_callbacks() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(RecordingCallback {
            events: events.clone(),
        }),
    )
    .expect("runtime init");

    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::Initialisiert",
        "HIKIM:3:2+Wartung+Am Wochenende",
        "HIKIM:4:2+Hinweis+Bitte lesen",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);

    handler.init().await.expect("dialog init replay response");

    let inst_messages = events
        .lock()
        .expect("callback event lock")
        .iter()
        .filter(|event| event.reason == CallbackReason::HaveInstMsg)
        .cloned()
        .collect::<Vec<_>>();

    assert_eq!(inst_messages.len(), 2);
    assert_eq!(inst_messages[0].message, "Wartung: Am Wochenende");
    assert_eq!(inst_messages[0].data_type, CallbackDataType::None);
    assert_eq!(inst_messages[0].current_value, None);
    assert_eq!(inst_messages[1].message, "Hinweis: Bitte lesen");
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_init_rejects_mismatched_response_reference() {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response_for_request(
        "WRONG",
        1,
        &["HIRMG:2:2+0010::Initialisiert"],
    ))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);

    let err = handler
        .init()
        .await
        .expect_err("wrong response reference is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::Protocol);
    assert!(handler.dialog_context().dialog_id.is_none());
}

#[tokio::test]
async fn handler_execute_uses_dialog_context_from_init_response() {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&[
            "HIRMG:2:2+0010::Initialisiert",
            "HIUPD:3:6+0001234567::280:12345678+DE02123456780000000000+customer+1+EUR+Max Mustermann++Girokonto",
        ])),
        Ok(custom_msg_response_for_request(
            "DIALOG1",
            2,
            &[
                "HIRMG:2:2+0010::OK",
                "HISAL:3:7+DE02123456780000000000:MARKDEF1100+Girokonto+EUR+C:123,45:EUR:20260605",
            ],
        )),
    ]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());

    handler.init().await.expect("dialog init replay response");
    let init_status = handler
        .dialog_status()
        .init_status
        .as_ref()
        .expect("handler keeps dialog init status");
    assert_eq!(
        init_status.global_status.successes()[0].text,
        "Initialisiert"
    );
    let job = handler.new_job("SaldoReq").expect("job is in registry");
    handler.add_to_queue(job);

    let status = handler.execute().await.expect("custom message response");

    assert!(status.success);
    assert_eq!(status.customer_ids(), vec!["customer"]);
    let dialog_status = status
        .dialog_status("customer")
        .expect("execute result carries current dialog status");
    assert!(dialog_status.init_status.is_some());
    assert_eq!(dialog_status.message_statuses.len(), 1);
    assert_eq!(
        dialog_status.message_statuses[0].global_status.successes()[0].text,
        "OK"
    );
    assert!(dialog_status.end_status.is_none());
    assert!(!status.is_ok());
    assert_eq!(handler.dialog_context().message_number, 3);
    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 2);

    let init_body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert!(init_body.contains("+300+0+1'"));

    let execute_body = String::from_utf8(requests[1].body.clone()).expect("request body is text");
    assert!(execute_body.contains("+300+DIALOG1+2'"));
    assert!(execute_body.contains("HKSAL:2:7+DE02123456780000000000::0001234567::280:12345678+N'"));
    assert!(execute_body.ends_with("HNHBS:3:1+2'"));
}

#[tokio::test]
async fn handler_execute_rejects_mismatched_response_reference() {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        accounts: vec![giro_account()],
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&["HIRMG:2:2+0010::Initialisiert"])),
        Ok(custom_msg_response(&[
            "HIRMG:2:2+0010::OK",
            "HISAL:3:7+DE02123456780000000000:MARKDEF1100+Girokonto+EUR+C:123,45:EUR:20260605",
        ])),
    ]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);

    handler.init().await.expect("dialog init replay response");
    let job = handler.new_job("SaldoReq").expect("job is in registry");
    handler.add_to_queue(job);

    let err = handler
        .execute()
        .await
        .expect_err("wrong custom message response reference is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::Protocol);
    assert_eq!(handler.queued_jobs().len(), 1);
}

#[tokio::test]
async fn handler_execute_rejects_mismatched_response_dialog_id() {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        accounts: vec![giro_account()],
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&["HIRMG:2:2+0010::Initialisiert"])),
        Ok(fints_response(
            "OTHERDIALOG",
            1,
            "DIALOG1",
            2,
            &[
                "HIRMG:2:2+0010::OK",
                "HISAL:3:7+DE02123456780000000000:MARKDEF1100+Girokonto+EUR+C:123,45:EUR:20260605",
            ],
        )),
    ]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);

    handler.init().await.expect("dialog init replay response");
    let job = handler.new_job("SaldoReq").expect("job is in registry");
    handler.add_to_queue(job);

    let err = handler
        .execute()
        .await
        .expect_err("wrong custom message response dialog id is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::Protocol);
    assert_eq!(handler.queued_jobs().len(), 1);
    assert_eq!(
        handler.dialog_context().dialog_id.as_deref(),
        Some("DIALOG1")
    );
}

#[tokio::test]
async fn handler_close_sends_dialog_end_and_resets_context() {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&[
            "HIRMG:2:2+0010::Initialisiert",
            "HIUPD:3:6+0001234567::280:12345678+DE02123456780000000000+customer+1+EUR+Max Mustermann++Girokonto",
        ])),
        Ok(custom_msg_response_for_request(
            "DIALOG1",
            2,
            &[
                "HIRMG:2:2+0010::OK",
                "HISAL:3:7+DE02123456780000000000:MARKDEF1100+Girokonto+EUR+C:123,45:EUR:20260605",
            ],
        )),
        Ok(custom_msg_response_for_request(
            "DIALOG1",
            3,
            &["HIRMG:2:2+0010::Dialog beendet"],
        )),
    ]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());

    handler.init().await.expect("dialog init replay response");
    let job = handler.new_job("SaldoReq").expect("job is in registry");
    handler.add_to_queue(job);
    handler.execute().await.expect("custom message response");

    handler.close().await.expect("dialog end response");

    assert!(handler.dialog_context().dialog_id.is_none());
    assert_eq!(handler.dialog_context().message_number, 1);
    let end_status = handler
        .dialog_status()
        .end_status
        .as_ref()
        .expect("handler keeps dialog end status");
    assert_eq!(
        end_status.global_status.successes()[0].text,
        "Dialog beendet"
    );
    assert!(handler.dialog_status().is_ok());
    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 3);

    let close_body = String::from_utf8(requests[2].body.clone()).expect("request body is text");
    assert!(close_body.starts_with("HNHBK:1:3+"));
    assert!(close_body.contains("+300+DIALOG1+3'"));
    assert!(close_body.contains("HKEND:2:1+DIALOG1'"));
    assert!(close_body.ends_with("HNHBS:3:1+3'"));

    let size = &close_body["HNHBK:1:3+".len().."HNHBK:1:3+".len() + 12];
    assert_eq!(size, format!("{:012}", close_body.len()));
}

#[tokio::test]
async fn handler_close_preserves_context_on_dialog_end_error() {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&["HIRMG:2:2+0010::Initialisiert"])),
        Ok(custom_msg_response_for_request(
            "DIALOG1",
            2,
            &["HIRMG:2:2+9010::Dialogende fehlgeschlagen"],
        )),
    ]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());

    handler.init().await.expect("dialog init replay response");
    let err = handler
        .close()
        .await
        .expect_err("dialog end error is returned");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::Protocol);
    assert_eq!(
        handler.dialog_context().dialog_id.as_deref(),
        Some("DIALOG1")
    );
    assert_eq!(handler.dialog_context().message_number, 2);
    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 2);

    let close_body = String::from_utf8(requests[1].body.clone()).expect("request body is text");
    assert!(close_body.contains("+300+DIALOG1+2'"));
    assert!(close_body.contains("HKEND:2:1+DIALOG1'"));
}

#[tokio::test]
async fn handler_close_accepts_segment_error_when_global_status_is_ok_like_original() {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&["HIRMG:2:2+0010::Initialisiert"])),
        Ok(custom_msg_response_for_request(
            "DIALOG1",
            2,
            &[
                "HIRMG:2:2+0010::Dialog beendet",
                "HIRMS:3:2+9010:2:Segmentfehler",
            ],
        )),
    ]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);

    handler.init().await.expect("dialog init replay response");
    handler.close().await.expect("dialog end response");

    assert!(handler.dialog_context().dialog_id.is_none());
    let end_status = handler
        .dialog_status()
        .end_status
        .as_ref()
        .expect("dialog end status is kept");
    assert!(end_status.is_ok());
    assert_eq!(end_status.segment_status.errors()[0].text, "Segmentfehler");
}

#[tokio::test]
async fn handler_close_rejects_mismatched_response_dialog_id() {
    let passport = PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&["HIRMG:2:2+0010::Initialisiert"])),
        Ok(fints_response(
            "OTHERDIALOG",
            1,
            "DIALOG1",
            2,
            &["HIRMG:2:2+0010::Dialog beendet"],
        )),
    ]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);

    handler.init().await.expect("dialog init replay response");
    let err = handler
        .close()
        .await
        .expect_err("wrong dialog end response dialog id is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::Protocol);
    assert_eq!(
        handler.dialog_context().dialog_id.as_deref(),
        Some("DIALOG1")
    );
}

#[tokio::test]
async fn handler_uses_replay_comm_client() {
    let passport = PinTanPassport::new(PinTanPassportData {
        host: Some("https://fints.example.test/fints".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HIRMS:3:2+0020:2:Saldo bereitgestellt",
        "HISAL:4:7+DE02123456780000000000:MARKDEF1100+Girokonto+EUR+C:123,45:EUR:20260605+D:1,23:EUR:20260605+1000,00:EUR+900,00:EUR+100,00:EUR",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut job = handler.new_job("SaldoReq").expect("job is in registry");
    job.set_param("my.iban", "DE02123456780000000000");

    handler.add_to_queue(job);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert!(status.customer_ids().is_empty());
    assert_eq!(status.job_results[0].job_name, "SaldoReq");
    assert!(status.job_results[0].success);
    assert!(status.job_results[0].is_ok());
    assert_eq!(status.job_results[0].ret_number(), 1);
    assert_eq!(status.job_results[0].dialog_id(), Some("0"));
    assert_eq!(status.job_results[0].msg_num(), Some("1"));
    assert_eq!(status.job_results[0].seg_num(), Some("2"));
    assert_eq!(
        status.job_results[0].job_id_for_date("20260606"),
        "20260606/0/1/2"
    );
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.KTV.iban")
            .map(String::as_str),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.booked.BTG.value")
            .map(String::as_str),
        Some("123.45")
    );
    assert_eq!(status.job_results[0].global_return_values[0].code, "0010");
    assert_eq!(status.global_return_values[0].code, "0010");
    assert_eq!(status.job_results[0].ret_value(0).unwrap().code, "0020");
    assert_eq!(
        status.messages,
        vec![
            "0010:OK".to_owned(),
            "0020:Saldo bereitgestellt (2)".to_owned()
        ]
    );
    let Some(HbciJobResultData::SaldoReq(result)) = status.job_results[0].result.as_ref() else {
        panic!("expected SaldoReq result data");
    };
    let entry = &result.entries[0];
    assert_eq!(entry.konto.iban.as_deref(), Some("DE02123456780000000000"));
    assert_eq!(entry.konto.bic.as_deref(), Some("MARKDEF1100"));
    assert_eq!(entry.konto.account_type.as_deref(), Some("Girokonto"));
    assert_eq!(entry.konto.curr.as_deref(), Some("EUR"));
    assert_eq!(entry.ready.value.value, "123.45");
    assert_eq!(entry.ready.value.curr.as_deref(), Some("EUR"));
    assert_eq!(entry.ready.date.as_deref(), Some("2026-06-05"));
    assert_eq!(
        entry
            .unready
            .as_ref()
            .map(|saldo| saldo.value.value.as_str()),
        Some("-1.23")
    );
    assert_eq!(
        entry.kredit.as_ref().map(|value| value.value.as_str()),
        Some("1000.00")
    );
    assert_eq!(
        entry.available.as_ref().map(|value| value.value.as_str()),
        Some("900.00")
    );
    assert_eq!(
        entry.used.as_ref().map(|value| value.value.as_str()),
        Some("100.00")
    );
    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 1);

    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert!(body.starts_with("HNHBK:1:3+"));
    assert!(body.contains("HKSAL:2:7+DE02123456780000000000+N'"));
    assert!(body.ends_with("HNHBS:3:1+1'"));
    assert!(!body.contains("SaldoReq"));

    let size = &body["HNHBK:1:3+".len().."HNHBK:1:3+".len() + 12];
    assert_eq!(size, format!("{:012}", body.len()));
}

#[tokio::test]
async fn handler_renders_saldo_request_from_lowlevel_params_like_original() {
    let passport = PinTanPassport::new(PinTanPassportData {
        host: Some("https://fints.example.test/fints".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_ok_response())]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "SaldoReq",
            "params": {},
            "lowlevel_params": {
                "Saldo7.KTV.iban": "DE02123456780000000000",
                "Saldo7.KTV.bic": "MARKDEF1100",
                "Saldo7.allaccounts": "N",
                "Saldo7.maxentries": "7"
            }
        }"#,
    )
    .expect("job with lowlevel params");

    handler.add_to_queue(job);
    handler.execute().await.expect("replay response");

    let requests = replay.requests().expect("requests");
    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");

    assert!(body.contains("HKSAL:2:7+DE02123456780000000000:MARKDEF1100+N+7'"));
}

#[tokio::test]
async fn handler_rejects_saldo_request_without_account_fallback() {
    let passport = PinTanPassport::new(PinTanPassportData {
        host: Some("https://fints.example.test/fints".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_ok_response())]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let job = handler.new_job("SaldoReq").expect("job is in registry");

    handler.add_to_queue(job);
    let err = handler
        .execute()
        .await
        .expect_err("missing SaldoReq account is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(replay.requests().expect("requests").len(), 0);
}

#[tokio::test]
async fn handler_uses_passport_account_for_saldo_request() {
    let passport = PinTanPassport::new(PinTanPassportData {
        host: Some("https://fints.example.test/fints".to_owned()),
        accounts: vec![giro_account()],
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HISAL:3:7+DE02123456780000000000:MARKDEF1100+Girokonto+EUR+C:123,45:EUR:20260605",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let job = handler.new_job("SaldoReq").expect("job is in registry");

    handler.add_to_queue(job);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    let Some(HbciJobResultData::SaldoReq(result)) = status.job_results[0].result.as_ref() else {
        panic!("expected SaldoReq result data");
    };
    let entry = &result.entries[0];
    assert_eq!(entry.konto.number.as_deref(), Some("0001234567"));
    assert_eq!(entry.konto.blz.as_deref(), Some("12345678"));
    assert_eq!(entry.konto.country.as_deref(), Some("DE"));

    let requests = replay.requests().expect("requests");
    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");

    assert!(
        body.contains("HKSAL:2:7+DE02123456780000000000:MARKDEF1100:0001234567::280:12345678+N'")
    );
}

#[tokio::test]
async fn handler_renders_repeated_saldo_requests() {
    let passport = PinTanPassport::new(PinTanPassportData {
        host: Some("https://fints.example.test/fints".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HISAL:3:7+DE02123456780000000000:MARKDEF1100+Girokonto+EUR+C:123,45:EUR:20260605",
        "HISAL:4:7+DE02123456780000000001:MARKDEF1100+Sparkonto+EUR+C:987,65:EUR:20260605",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut first = handler.new_job("SaldoReq").expect("job is in registry");
    first.set_param("my.iban", "DE02123456780000000000");
    let mut second = handler.new_job("SaldoReq").expect("job is in registry");
    second.set_param("my.iban", "DE02123456780000000001");

    handler.add_to_queue(first);
    handler.add_to_queue(second);
    let status = handler.execute().await.expect("replay response");

    let Some(HbciJobResultData::SaldoReq(first_result)) = status.job_results[0].result.as_ref()
    else {
        panic!("expected first SaldoReq result data");
    };
    let Some(HbciJobResultData::SaldoReq(second_result)) = status.job_results[1].result.as_ref()
    else {
        panic!("expected second SaldoReq result data");
    };
    assert_eq!(
        first_result.entries[0].konto.iban.as_deref(),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        second_result.entries[0].konto.iban.as_deref(),
        Some("DE02123456780000000001")
    );
    assert_eq!(first_result.entries[0].ready.value.value, "123.45");
    assert_eq!(second_result.entries[0].ready.value.value, "987.65");

    let requests = replay.requests().expect("requests");
    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");

    assert!(body.contains("HKSAL:2:7+DE02123456780000000000+N'"));
    assert!(body.contains("HKSAL:3:7+DE02123456780000000001+N'"));
    assert!(body.ends_with("HNHBS:4:1+1'"));
}

#[tokio::test]
async fn handler_renders_saldo_request_all_without_account() {
    let passport = PinTanPassport::new(PinTanPassportData {
        host: Some("https://fints.example.test/fints".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HISAL:3:7+DE02123456780000000000:MARKDEF1100+Girokonto+EUR+C:123,45:EUR:20260605",
        "HISAL:4:7+DE02123456780000000001:MARKDEF1100+Sparkonto+EUR+C:987,65:EUR:20260605",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let job = handler.new_job("SaldoReqAll").expect("job is in registry");

    handler.add_to_queue(job);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "SaldoReqAll");
    let Some(HbciJobResultData::SaldoReq(result)) = status.job_results[0].result.as_ref() else {
        panic!("expected SaldoReqAll to reuse SaldoReq result data");
    };
    assert_eq!(result.entries.len(), 2);
    assert_eq!(
        result.entries[0].konto.iban.as_deref(),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        result.entries[1].konto.iban.as_deref(),
        Some("DE02123456780000000001")
    );
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.KTV.iban")
            .map(String::as_str),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content_2.KTV.iban")
            .map(String::as_str),
        Some("DE02123456780000000001")
    );

    let requests = replay.requests().expect("requests");
    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");

    assert!(body.contains("HKSAL:2:7++J'"));
}

#[tokio::test]
async fn handler_marks_segment_return_errors_as_failed_jobs() {
    let passport = PinTanPassport::new(PinTanPassportData {
        host: Some("https://fints.example.test/fints".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HIRMS:3:2+9010:2:Saldo abgelehnt",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut job = handler.new_job("SaldoReq").expect("job is in registry");
    job.set_param("my.iban", "DE02123456780000000000");

    handler.add_to_queue(job);
    let status = handler.execute().await.expect("replay response");

    assert!(!status.success);
    assert!(!status.job_results[0].success);
    assert!(status.job_results[0].result.is_none());
    assert_eq!(status.segment_return_values[0].code, "9010");
    assert!(status.segment_return_values[0].is_error());
    assert_eq!(
        status.messages,
        vec!["0010:OK".to_owned(), "9010:Saldo abgelehnt (2)".to_owned()]
    );
}
