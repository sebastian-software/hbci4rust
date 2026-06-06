use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use hbci4rust::{
    CallbackEvent, CallbackResponse, CommResponse, HbciCallback, HbciHandler, HbciJobResultData,
    HbciResult, Konto, Limit, PassportStorage, PinTanPassport, PinTanPassportData,
    ReplayCommClient, Value, init,
    protocol::{load_protocol_spec, parse_wire_message},
};

#[derive(Debug)]
struct TestCallback;

#[async_trait]
impl HbciCallback for TestCallback {
    async fn handle(&self, _event: CallbackEvent) -> HbciResult<CallbackResponse> {
        Ok(CallbackResponse::empty())
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
    let job = handler.new_job("SaldoReq").expect("job is in registry");
    handler.add_to_queue(job);

    let status = handler.execute().await.expect("custom message response");

    assert!(status.success);
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
    init(BTreeMap::<String, String>::new(), Arc::new(TestCallback)).expect("runtime init");

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
    assert_eq!(status.job_results[0].job_name, "SaldoReq");
    assert!(status.job_results[0].success);
    assert_eq!(status.global_return_values[0].code, "0010");
    assert_eq!(status.job_results[0].return_values[0].code, "0020");
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
