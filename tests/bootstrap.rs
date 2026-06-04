use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use hbci4rust::{
    CallbackEvent, CallbackResponse, CommResponse, HbciCallback, HbciHandler, HbciJobResultData,
    HbciResult, PassportStorage, PinTanPassport, PinTanPassportData, ReplayCommClient, init,
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
    let mut body = "HNHBK:1:3+000000000123+300+DIALOG1+1+DIALOG0:1'".to_owned();
    for segment in body_segments {
        body.push_str(segment);
        body.push('\'');
    }
    body.push_str("HNHBS:");
    body.push_str(&(body_segments.len() + 2).to_string());
    body.push_str(":1+1'");
    CommResponse::ok(body)
}

fn custom_msg_ok_response() -> CommResponse {
    custom_msg_response(&["HIRMG:2:2+0010::OK"])
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
    };

    let bytes = PassportStorage::save_to_vec(&data, b"correct horse battery staple")
        .expect("passport saves");
    let restored = PassportStorage::load_from_slice(&bytes, b"correct horse battery staple")
        .expect("passport loads");

    assert_eq!(restored, data);
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
async fn handler_rejects_saldo_request_without_iban() {
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
