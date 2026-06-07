use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use hbci4rust::{
    CallbackEvent, CallbackReason, CallbackResponse, CommResponse, HbciCallback, HbciHandler,
    HbciResult, Konto, PinTanPassport, PinTanPassportData, ReplayCommClient, done, init,
};

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

#[tokio::test]
async fn handler_emits_connection_callbacks_for_init_execute_and_close() {
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(RecordingCallback {
            events: events.clone(),
        }),
    )
    .expect("runtime init");

    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&["HIRMG:2:2+0010::Initialisiert"])),
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
    let mut handler = HbciHandler::with_comm(
        "300",
        passport_with_cached_pin(PinTanPassportData {
            accounts: vec![giro_account()],
            ..signed_pintan_data()
        }),
        replay.clone(),
    );

    handler.init().await.expect("dialog init replay response");
    let job = handler.new_job("SaldoReq").expect("job is in registry");
    handler.add_to_queue(job);
    handler.execute().await.expect("custom message response");
    handler.close().await.expect("dialog end response");

    let reasons = events
        .lock()
        .expect("callback event lock")
        .iter()
        .map(|event| event.reason)
        .collect::<Vec<_>>();
    assert_eq!(
        reasons,
        [
            CallbackReason::NeedConnection,
            CallbackReason::CloseConnection,
            CallbackReason::NeedConnection,
            CallbackReason::CloseConnection,
            CallbackReason::NeedConnection,
            CallbackReason::CloseConnection,
        ]
    );
    assert_eq!(replay.requests().expect("requests").len(), 3);
    done().expect("runtime reset");
}

fn passport_with_cached_pin(data: PinTanPassportData) -> PinTanPassport {
    let mut passport = PinTanPassport::new(data);
    passport.set_pin("12345");
    passport
}

fn signed_pintan_data() -> PinTanPassportData {
    PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    }
}

fn giro_account() -> Konto {
    Konto {
        country: Some("DE".to_owned()),
        blz: Some("12345678".to_owned()),
        number: Some("0001234567".to_owned()),
        bic: Some("MARKDEF1100".to_owned()),
        iban: Some("DE02123456780000000000".to_owned()),
        curr: Some("EUR".to_owned()),
        ..Konto::default()
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
    let mut body = format!("HNHBK:1:3+000000000123+300+DIALOG1+1+{ref_dialog_id}:{ref_msgnum}'");
    for segment in body_segments {
        body.push_str(segment);
        body.push('\'');
    }
    body.push_str("HNHBS:");
    body.push_str(&(body_segments.len() + 2).to_string());
    body.push_str(":1+1'");
    CommResponse::ok(body)
}
