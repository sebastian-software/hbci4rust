use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use hbci4rust::{
    CallbackDataType, CallbackEvent, CallbackReason, CallbackResponse, ChallengeInfo, CommResponse,
    HbciCallback, HbciHandler, HbciJobResultData, HbciMsgStatus, HbciResult, HbciReturnValue,
    HbciStatus, Konto, Limit, OrderHashMode, PassportStorage, PinTanPassport, PinTanPassportData,
    ReplayCommClient, TanMethodSelection, UserSig, Value, done, init,
    protocol::{load_protocol_spec, parse_wire_message},
    sepa::{CAMT_052_001_01_URN, PAIN_001_001_02_URN},
};

static RUNTIME_CALLBACK_TEST_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
const CHALLENGE_DATA: &str = include_str!("fixtures/hbci4java/secmech/challengedata.xml");

fn pintan_bpd(can1step: &str, mechanisms: &[(&str, &str)]) -> BTreeMap<String, String> {
    let mut props = BTreeMap::from([(
        "Params.TAN2StepPar5.ParTAN2Step.can1step".to_owned(),
        can1step.to_owned(),
    )]);

    for (index, (secfunc, name)) in mechanisms.iter().enumerate() {
        let prefix = format!("Params_{}.TAN2StepPar5.ParTAN2Step", index + 1);
        props.insert(format!("{prefix}.secfunc"), (*secfunc).to_owned());
        props.insert(format!("{prefix}.name"), (*name).to_owned());
    }

    props
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

#[derive(Debug)]
struct FixedTanCallback {
    events: Arc<Mutex<Vec<CallbackEvent>>>,
    tan: String,
}

#[async_trait]
impl HbciCallback for FixedTanCallback {
    async fn handle(&self, event: CallbackEvent) -> HbciResult<CallbackResponse> {
        let reason = event.reason;
        self.events.lock().expect("callback event lock").push(event);
        if reason == CallbackReason::NeedPtTan {
            Ok(CallbackResponse::value(self.tan.clone()))
        } else {
            Ok(CallbackResponse::empty())
        }
    }
}

#[derive(Debug)]
struct SecmechSelectingCallback {
    events: Arc<Mutex<Vec<CallbackEvent>>>,
    selection: String,
}

#[async_trait]
impl HbciCallback for SecmechSelectingCallback {
    async fn handle(&self, event: CallbackEvent) -> HbciResult<CallbackResponse> {
        let reason = event.reason;
        self.events.lock().expect("callback event lock").push(event);
        if reason == CallbackReason::NeedPtSecMech {
            Ok(CallbackResponse::value(self.selection.clone()))
        } else {
            Ok(CallbackResponse::empty())
        }
    }
}

#[derive(Debug)]
struct TanMediaSelectingCallback {
    events: Arc<Mutex<Vec<CallbackEvent>>>,
    selection: Option<String>,
}

#[async_trait]
impl HbciCallback for TanMediaSelectingCallback {
    async fn handle(&self, event: CallbackEvent) -> HbciResult<CallbackResponse> {
        let reason = event.reason;
        self.events.lock().expect("callback event lock").push(event);
        if reason == CallbackReason::NeedPtTanMedia {
            Ok(self
                .selection
                .as_ref()
                .map(|value| CallbackResponse::value(value.clone()))
                .unwrap_or_else(CallbackResponse::empty))
        } else {
            Ok(CallbackResponse::empty())
        }
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

fn assert_signed_dialog_init_request(
    body: &str,
    customer_id: &str,
    bpd_version: &str,
    upd_version: &str,
) {
    assert!(body.starts_with("HNHBK:1:3+"), "{body}");
    assert!(body.contains("+300+0+1'"), "{body}");
    assert!(body.contains("HNSHK:2:4+PIN:1+999+"), "{body}");
    assert!(
        body.contains(&format!("HKIDN:3:2+280:12345678+{customer_id}+0+0'")),
        "{body}"
    );
    assert!(
        body.contains(&format!(
            "HKVVB:4:3+{bpd_version}+{upd_version}+0+hbci4rust+0.1.0'"
        )),
        "{body}"
    );
    assert!(body.ends_with("HNHBS:6:1+1'"), "{body}");

    let sig_head = fints_segment(body, "HNSHK");
    let sig_tail = fints_segment(body, "HNSHA");
    let sig_head_checkref = sig_head
        .split('+')
        .nth(3)
        .expect("HNSHK has check reference");
    let sig_tail_parts = sig_tail.split('+').collect::<Vec<_>>();

    assert_eq!(sig_tail_parts.get(1).copied(), Some(sig_head_checkref));
    assert_eq!(sig_tail_parts.get(2).copied(), Some(""));
    assert_eq!(sig_tail_parts.get(3).copied(), Some("12345"));

    let size = &body["HNHBK:1:3+".len().."HNHBK:1:3+".len() + 12];
    assert_eq!(size, format!("{:012}", body.len()));
}

fn assert_signed_dialog_end_request(body: &str, dialog_id: &str, msgnum: &str) {
    assert!(body.starts_with("HNHBK:1:3+"), "{body}");
    assert!(
        body.contains(&format!("+300+{dialog_id}+{msgnum}'")),
        "{body}"
    );
    assert!(body.contains("HNSHK:2:4+PIN:"), "{body}");
    assert!(body.contains(&format!("HKEND:3:1+{dialog_id}'")), "{body}");
    assert!(body.ends_with(&format!("HNHBS:5:1+{msgnum}'")), "{body}");

    let sig_head = fints_segment(body, "HNSHK");
    let sig_tail = fints_segment(body, "HNSHA");
    let sig_head_checkref = sig_head
        .split('+')
        .nth(3)
        .expect("HNSHK has check reference");
    let sig_tail_parts = sig_tail.split('+').collect::<Vec<_>>();

    assert_eq!(sig_tail_parts.get(1).copied(), Some(sig_head_checkref));
    assert_eq!(sig_tail_parts.get(2).copied(), Some(""));
    assert_eq!(sig_tail_parts.get(3).copied(), Some("12345"));

    let size = &body["HNHBK:1:3+".len().."HNHBK:1:3+".len() + 12];
    assert_eq!(size, format!("{:012}", body.len()));
}

fn assert_signed_custom_msg_request(body: &str, dialog_id: &str, msgnum: &str, tail_seq: usize) {
    assert_signed_custom_msg_request_bytes(body.as_bytes(), dialog_id, msgnum, tail_seq);
}

fn assert_signed_custom_msg_request_bytes(
    body: &[u8],
    dialog_id: &str,
    msgnum: &str,
    tail_seq: usize,
) {
    let text = String::from_utf8_lossy(body);
    assert!(text.starts_with("HNHBK:1:3+"), "{text}");
    assert!(
        text.contains(&format!("+300+{dialog_id}+{msgnum}'")),
        "{text}"
    );
    assert!(text.contains("HNSHK:2:4+PIN:"), "{text}");
    assert!(
        text.contains(&format!("HNSHA:{}:2+", tail_seq - 1)),
        "{text}"
    );
    assert!(
        text.ends_with(&format!("HNHBS:{tail_seq}:1+{msgnum}'")),
        "{text}"
    );

    let sig_head = fints_segment(&text, "HNSHK");
    let sig_tail = fints_segment(&text, "HNSHA");
    let sig_head_checkref = sig_head
        .split('+')
        .nth(3)
        .expect("HNSHK has check reference");
    let sig_tail_parts = sig_tail.split('+').collect::<Vec<_>>();

    assert_eq!(sig_tail_parts.get(1).copied(), Some(sig_head_checkref));
    assert_eq!(sig_tail_parts.get(2).copied(), Some(""));
    assert_eq!(sig_tail_parts.get(3).copied(), Some("12345"));

    let size_start = "HNHBK:1:3+".len();
    let size = std::str::from_utf8(&body[size_start..size_start + 12]).expect("size is ASCII");
    assert_eq!(size, format!("{:012}", body.len()));
}

fn fints_segment<'a>(body: &'a str, code: &str) -> &'a str {
    body.split('\'')
        .find(|segment| segment.starts_with(code))
        .unwrap_or_else(|| panic!("{code} segment missing in {body}"))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn custom_msg_ok_response() -> CommResponse {
    custom_msg_response(&["HIRMG:2:2+0010::OK"])
}

fn kums_response_segment(code: &str, booked: &str, notbooked: &str) -> String {
    format!(
        "{code}:3:7+@{}@{}+@{}@{}",
        booked.len(),
        booked,
        notbooked.len(),
        notbooked
    )
}

fn kums_camt_response_segment(booked_first: &str, booked_second: &str, notbooked: &str) -> String {
    let escaped_format = CAMT_052_001_01_URN.replace(':', "?:");
    format!(
        "HICAZ:3:1+DE02123456780000000000:MARKDEF1100+{}+@{}@{}:@{}@{}+@{}@{}",
        escaped_format,
        booked_first.len(),
        booked_first,
        booked_second.len(),
        booked_second,
        notbooked.len(),
        notbooked
    )
}

fn mt940_booked_payload() -> String {
    concat!(
        "\r\n:20:STARTUMS",
        "\r\n:25:12345678/1234567890",
        "\r\n:28C:1",
        "\r\n:60F:C230209EUR100,00",
        "\r\n:61:2302090209CR2,00NTRFBOOKEDREF",
        "\r\n:86:152?00GUTSCHRIFT M[LLER?109245?20Booked usage?32Max Mustermann?34000",
        "\r\n:62F:C230209EUR102,00",
        "\r\n-"
    )
    .to_owned()
}

fn mt942_unbooked_payload() -> String {
    concat!(
        "\r\n:20:STARTUMV",
        "\r\n:25:12345678/1234567890",
        "\r\n:28C:2",
        "\r\n:60F:C230210EUR0,00",
        "\r\n:61:2302100210CR3,00NTRFUNBOOKEDREF",
        "\r\n:86:152?00VORMERKUNG?20Unbooked usage",
        "\r\n:62F:C230210EUR3,00",
        "\r\n-"
    )
    .to_owned()
}

fn camt_payload(name: &str) -> String {
    format!("<Document><BkToCstmrAcctRpt>{name}</BkToCstmrAcctRpt></Document>")
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
        creditorid: None,
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
fn acc_info_exposes_original_near_v2_constraints() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let job = handler.new_job("AccInfo").expect("job is in registry");

    assert_eq!(job.constraints().len(), 5);
    assert_eq!(
        job.constraint("my.country")
            .expect("country constraint")
            .destination_name,
        "AccInfo2.KTV.KIK.country"
    );
    assert_eq!(
        job.constraint("my.country")
            .expect("country constraint")
            .default_value
            .as_deref(),
        Some("DE")
    );
    assert_eq!(
        job.constraint("my.blz")
            .expect("bank code constraint")
            .destination_name,
        "AccInfo2.KTV.KIK.blz"
    );
    assert_eq!(
        job.constraint("my.number")
            .expect("account number constraint")
            .destination_name,
        "AccInfo2.KTV.number"
    );
    assert_eq!(
        job.constraint("my.subnumber")
            .expect("subnumber constraint")
            .default_value
            .as_deref(),
        Some("")
    );
    assert_eq!(
        job.constraint("all")
            .expect("allaccounts constraint")
            .destination_name,
        "AccInfo2.allaccounts"
    );
    assert_eq!(
        job.constraint("all")
            .expect("allaccounts constraint")
            .default_value
            .as_deref(),
        Some("N")
    );
}

#[test]
fn dauer_sepa_list_exposes_original_near_v2_constraints() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let job = handler
        .new_job("DauerSEPAList")
        .expect("job is in registry");

    assert_eq!(job.constraints().len(), 11);
    assert_eq!(
        job.constraint("my.iban")
            .expect("iban constraint")
            .destination_name,
        "DauerSEPAList2.My.iban"
    );
    assert_eq!(
        job.constraint("src.iban")
            .expect("source iban alias")
            .destination_name,
        "DauerSEPAList2.My.iban"
    );
    assert_eq!(
        job.constraint("src.bic")
            .expect("source bic alias")
            .destination_name,
        "DauerSEPAList2.My.bic"
    );
    assert_eq!(
        job.constraint("_sepadescriptor")
            .expect("sepa descriptor")
            .default_value
            .as_deref(),
        Some(PAIN_001_001_02_URN)
    );
    assert_eq!(
        job.constraint("maxentries")
            .expect("maxentries constraint")
            .destination_name,
        "DauerSEPAList2.maxentries"
    );
}

#[test]
fn dauer_sepa_new_exposes_original_near_v1_constraints() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let job = handler.new_job("DauerSEPANew").expect("job is in registry");

    assert_eq!(job.constraints().len(), 26);
    assert_eq!(
        job.constraint("src.iban")
            .expect("source iban constraint")
            .destination_name,
        "DauerSEPANew1.My.iban"
    );
    assert_eq!(
        job.constraint("src.bic")
            .expect("source bic constraint")
            .destination_name,
        "DauerSEPANew1.My.bic"
    );
    assert_eq!(
        job.constraint("_sepadescriptor")
            .expect("sepa descriptor")
            .default_value
            .as_deref(),
        Some(PAIN_001_001_02_URN)
    );
    assert_eq!(
        job.constraint("_sepapain")
            .expect("sepa pain")
            .destination_name,
        "DauerSEPANew1.sepapain"
    );
    assert_eq!(
        job.constraint("firstdate")
            .expect("firstdate constraint")
            .destination_name,
        "DauerSEPANew1.DauerDetails.firstdate"
    );
    assert_eq!(
        job.constraint("lastdate")
            .expect("lastdate constraint")
            .default_value
            .as_deref(),
        Some("")
    );
    assert_eq!(
        job.constraint("endtoendid")
            .expect("endtoendid dummy constraint")
            .default_value
            .as_deref(),
        Some("NOTPROVIDED")
    );
}

#[test]
fn dauer_sepa_edit_exposes_original_near_v1_constraints() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let job = handler
        .new_job("DauerSEPAEdit")
        .expect("job is in registry");

    assert_eq!(job.constraints().len(), 28);
    assert_eq!(
        job.constraint("src.iban")
            .expect("source iban constraint")
            .destination_name,
        "DauerSEPAEdit1.My.iban"
    );
    assert_eq!(
        job.constraint("_sepapain")
            .expect("sepa pain")
            .destination_name,
        "DauerSEPAEdit1.sepapain"
    );
    assert_eq!(
        job.constraint("orderid")
            .expect("orderid constraint")
            .destination_name,
        "DauerSEPAEdit1.orderid"
    );
    assert_eq!(
        job.constraint("orderid")
            .expect("orderid constraint")
            .default_value
            .as_deref(),
        None
    );
    assert_eq!(
        job.constraint("date")
            .expect("date constraint")
            .destination_name,
        "DauerSEPAEdit1.date"
    );
    assert_eq!(
        job.constraint("date")
            .expect("date constraint")
            .default_value
            .as_deref(),
        Some("")
    );
    assert_eq!(
        job.constraint("firstdate")
            .expect("firstdate constraint")
            .destination_name,
        "DauerSEPAEdit1.DauerDetails.firstdate"
    );
    assert_eq!(
        job.constraint("endtoendid")
            .expect("endtoendid dummy constraint")
            .default_value
            .as_deref(),
        Some("NOTPROVIDED")
    );
}

#[test]
fn dauer_sepa_del_exposes_original_near_v1_constraints() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let job = handler.new_job("DauerSEPADel").expect("job is in registry");

    assert_eq!(job.constraints().len(), 28);
    assert_eq!(
        job.constraint("src.iban")
            .expect("source iban constraint")
            .destination_name,
        "DauerSEPADel1.My.iban"
    );
    assert_eq!(
        job.constraint("_sepapain")
            .expect("sepa pain")
            .destination_name,
        "DauerSEPADel1.sepapain"
    );
    assert_eq!(
        job.constraint("orderid")
            .expect("orderid constraint")
            .destination_name,
        "DauerSEPADel1.orderid"
    );
    assert_eq!(
        job.constraint("date")
            .expect("date constraint")
            .destination_name,
        "DauerSEPADel1.date"
    );
    assert_eq!(
        job.constraint("firstdate")
            .expect("firstdate constraint")
            .destination_name,
        "DauerSEPADel1.DauerDetails.firstdate"
    );
    assert_eq!(
        job.constraint("endtoendid")
            .expect("endtoendid dummy constraint")
            .default_value
            .as_deref(),
        Some("NOTPROVIDED")
    );
}

#[test]
fn kums_all_exposes_original_near_constraints() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let kums = handler.new_job("KUmsAll").expect("job is in registry");

    assert_eq!(kums.constraints().len(), 10);
    assert_eq!(
        kums.constraint("my.iban")
            .expect("iban constraint")
            .destination_name,
        "KUmsZeit7.KTV.iban"
    );
    assert_eq!(
        kums.constraint("my.country")
            .expect("country constraint")
            .default_value
            .as_deref(),
        Some("DE")
    );
    assert_eq!(
        kums.constraint("startdate")
            .expect("startdate constraint")
            .destination_name,
        "KUmsZeit7.startdate"
    );
    assert_eq!(
        kums.constraint("enddate")
            .expect("enddate constraint")
            .destination_name,
        "KUmsZeit7.enddate"
    );
    assert_eq!(
        kums.constraint("dummy")
            .expect("dummy allaccounts constraint")
            .default_value
            .as_deref(),
        Some("N")
    );
    assert_eq!(kums.constraint("offset"), None);
    assert_eq!(kums.constraint("my.curr"), None);
}

#[test]
fn kums_new_exposes_original_near_constraints() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let kums = handler.new_job("KUmsNew").expect("job is in registry");

    assert_eq!(kums.constraints().len(), 8);
    assert_eq!(
        kums.constraint("my.iban")
            .expect("iban constraint")
            .destination_name,
        "KUmsNew7.KTV.iban"
    );
    assert_eq!(
        kums.constraint("my.country")
            .expect("country constraint")
            .default_value
            .as_deref(),
        Some("DE")
    );
    assert_eq!(
        kums.constraint("maxentries")
            .expect("maxentries constraint")
            .destination_name,
        "KUmsNew7.maxentries"
    );
    assert_eq!(
        kums.constraint("dummyall")
            .expect("dummyall constraint")
            .default_value
            .as_deref(),
        Some("N")
    );
    assert_eq!(kums.constraint("startdate"), None);
    assert_eq!(kums.constraint("my.curr"), None);
}

#[test]
fn kums_all_camt_exposes_original_near_constraints() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let kums = handler.new_job("KUmsAllCamt").expect("job is in registry");

    assert_eq!(kums.constraints().len(), 12);
    assert_eq!(
        kums.constraint("my.iban")
            .expect("iban constraint")
            .destination_name,
        "KUmsZeitCamt1.KTV.iban"
    );
    assert_eq!(
        kums.constraint("suppformat")
            .expect("suppformat constraint")
            .destination_name,
        "KUmsZeitCamt1.formats.suppformat"
    );
    assert_eq!(
        kums.constraint("suppformat")
            .expect("suppformat constraint")
            .default_value
            .as_deref(),
        Some(CAMT_052_001_01_URN)
    );
    assert_eq!(
        kums.constraint("offset")
            .expect("offset constraint")
            .destination_name,
        "KUmsZeitCamt1.offset"
    );
    assert_eq!(
        kums.constraint("dummy")
            .expect("dummy allaccounts constraint")
            .default_value
            .as_deref(),
        Some("N")
    );
    assert_eq!(kums.constraint("my.curr"), None);
}

#[test]
fn sepa_info_exposes_original_near_empty_constraints() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut job = handler.new_job("SEPAInfo").expect("job is in registry");

    assert!(job.constraints().is_empty());
    let lowlevel = job.verify_constraints().expect("constraints resolve");
    assert!(lowlevel.is_empty());
}

#[test]
fn tan_media_list_exposes_original_near_v4_constraints() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut job = handler.new_job("TANMediaList").expect("job is in registry");

    assert_eq!(job.constraints().len(), 2);
    assert_eq!(
        job.constraint("mediatype")
            .expect("mediatype constraint")
            .destination_name,
        "TANMediaList4.mediatype"
    );
    assert_eq!(
        job.constraint("mediatype")
            .expect("mediatype constraint")
            .default_value
            .as_deref(),
        Some("0")
    );
    assert_eq!(
        job.constraint("mediacategory")
            .expect("mediacategory constraint")
            .destination_name,
        "TANMediaList4.mediacategory"
    );
    assert_eq!(
        job.constraint("mediacategory")
            .expect("mediacategory constraint")
            .default_value
            .as_deref(),
        Some("A")
    );
    let lowlevel = job.verify_constraints().expect("constraints resolve");
    assert_eq!(
        lowlevel.get("TANMediaList4.mediatype").map(String::as_str),
        Some("0")
    );
    assert_eq!(
        lowlevel
            .get("TANMediaList4.mediacategory")
            .map(String::as_str),
        Some("A")
    );
}

#[test]
fn tan2step_exposes_original_near_v5_constraints() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let handler = HbciHandler::new("300", passport);
    let mut hktan = handler.new_job("TAN2Step").expect("job is in registry");

    assert_eq!(hktan.constraints().len(), 24);
    assert_eq!(
        hktan
            .constraint("process")
            .expect("process constraint")
            .destination_name,
        "TAN2Step5.process"
    );
    assert_eq!(
        hktan
            .constraint("orderaccount.country")
            .expect("order account country constraint")
            .default_value
            .as_deref(),
        Some("DE")
    );
    assert_eq!(
        hktan
            .constraint("ChallengeKlassParam9")
            .expect("challenge param 9 constraint")
            .destination_name,
        "TAN2Step5.ChallengeKlassParams.param9"
    );

    hktan
        .try_set_param("orderhash", "12345")
        .expect("orderhash is accepted");
    assert_eq!(hktan.param("orderhash"), Some("12345"));
    assert_eq!(hktan.lowlevel_param("TAN2Step5.orderhash"), Some("B12345"));
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
fn checked_date_param_setter_uses_original_iso_shape() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "DateJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "range.startdate",
                    "destination_name": "DateSeg.range.startdate",
                    "default_value": null,
                    "indexed": false
                }
            ]
        }"#,
    )
    .expect("date job");

    job.try_set_param_date("range.startdate", " 2024-02-29 ")
        .expect("date parameter is accepted");

    assert_eq!(job.param("range.startdate"), Some("2024-02-29"));
    assert_eq!(
        job.lowlevel_param("DateSeg.range.startdate"),
        Some("2024-02-29")
    );
}

#[test]
fn checked_date_param_setter_rejects_invalid_iso_date() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "DateJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "range.startdate",
                    "destination_name": "DateSeg.range.startdate",
                    "default_value": null,
                    "indexed": false
                }
            ]
        }"#,
    )
    .expect("date job");

    let err = job
        .try_set_param_date("range.startdate", "2023-02-29")
        .expect_err("invalid date is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert!(err.message().contains("Date day"));
    assert!(job.lowlevel_params().is_empty());
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
fn indexed_date_param_setter_inserts_index_like_original() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "IndexedDateJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "range.startdate",
                    "destination_name": "DateSeg.ranges.date",
                    "default_value": null,
                    "indexed": true
                }
            ]
        }"#,
    )
    .expect("indexed date job");

    job.try_set_indexed_param_date("range.startdate", 2, "2026-06-06")
        .expect("indexed date parameter is accepted");

    assert_eq!(
        job.lowlevel_param("DateSeg.ranges.date[2]"),
        Some("2026-06-06")
    );
    assert_eq!(job.param("range.startdate"), None);
}

#[test]
fn indexed_date_param_setter_rejects_invalid_iso_date_before_writing() {
    let mut job: hbci4rust::HbciJob = serde_json::from_str(
        r#"{
            "name": "IndexedDateJob",
            "params": {},
            "constraints": [
                {
                    "frontend_name": "range.startdate",
                    "destination_name": "DateSeg.ranges.date",
                    "default_value": null,
                    "indexed": true
                }
            ]
        }"#,
    )
    .expect("indexed date job");

    let err = job
        .try_set_indexed_param_date("range.startdate", 2, "2026-13-06")
        .expect_err("invalid indexed date is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert!(err.message().contains("Date month"));
    assert!(job.lowlevel_params().is_empty());
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

#[test]
fn checked_queue_add_prepares_kums_all_like_original_add_task() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let mut handler = HbciHandler::new("300", passport);
    let mut kums = handler.new_job("KUmsAll").expect("job is in registry");
    kums.set_param_account("my", &giro_account());
    kums.try_set_param_date("startdate", "2026-06-01")
        .expect("start date is accepted");
    kums.try_set_param_date("enddate", "2026-06-06")
        .expect("end date is accepted");
    kums.try_set_param_int("maxentries", 25)
        .expect("maxentries is accepted");

    handler
        .try_add_to_queue(kums)
        .expect("valid KUmsAll job is accepted");

    let queued = &handler.queued_jobs()[0];
    assert_eq!(
        queued.lowlevel_param("KUmsZeit7.KTV.iban"),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        queued.lowlevel_param("KUmsZeit7.startdate"),
        Some("2026-06-01")
    );
    assert_eq!(
        queued.lowlevel_param("KUmsZeit7.enddate"),
        Some("2026-06-06")
    );
    assert_eq!(queued.lowlevel_param("KUmsZeit7.maxentries"), Some("25"));
    assert_eq!(queued.lowlevel_param("KUmsZeit7.allaccounts"), Some("N"));
}

#[test]
fn checked_queue_add_prepares_kums_new_like_original_add_task() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let mut handler = HbciHandler::new("300", passport);
    let mut kums = handler.new_job("KUmsNew").expect("job is in registry");
    kums.set_param_account("my", &giro_account());
    kums.try_set_param_int("maxentries", 25)
        .expect("maxentries is accepted");

    handler
        .try_add_to_queue(kums)
        .expect("valid KUmsNew job is accepted");

    let queued = &handler.queued_jobs()[0];
    assert_eq!(
        queued.lowlevel_param("KUmsNew7.KTV.iban"),
        Some("DE02123456780000000000")
    );
    assert_eq!(queued.lowlevel_param("KUmsNew7.maxentries"), Some("25"));
    assert_eq!(queued.lowlevel_param("KUmsNew7.allaccounts"), Some("N"));
}

#[test]
fn checked_queue_add_prepares_kums_all_camt_like_original_add_task() {
    let passport = PinTanPassport::new(PinTanPassportData::default());
    let mut handler = HbciHandler::new("300", passport);
    let mut kums = handler.new_job("KUmsAllCamt").expect("job is in registry");
    kums.set_param_account("my", &giro_account());
    kums.try_set_param_date("startdate", "2026-06-01")
        .expect("start date is accepted");
    kums.try_set_param_date("enddate", "2026-06-06")
        .expect("end date is accepted");
    kums.try_set_param_int("maxentries", 25)
        .expect("maxentries is accepted");
    kums.set_param("offset", "CURSOR");

    handler
        .try_add_to_queue(kums)
        .expect("valid KUmsAllCamt job is accepted");

    let queued = &handler.queued_jobs()[0];
    assert_eq!(
        queued.lowlevel_param("KUmsZeitCamt1.KTV.iban"),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        queued.lowlevel_param("KUmsZeitCamt1.formats.suppformat"),
        Some(CAMT_052_001_01_URN)
    );
    assert_eq!(
        queued.lowlevel_param("KUmsZeitCamt1.startdate"),
        Some("2026-06-01")
    );
    assert_eq!(
        queued.lowlevel_param("KUmsZeitCamt1.enddate"),
        Some("2026-06-06")
    );
    assert_eq!(
        queued.lowlevel_param("KUmsZeitCamt1.maxentries"),
        Some("25")
    );
    assert_eq!(
        queued.lowlevel_param("KUmsZeitCamt1.offset"),
        Some("CURSOR")
    );
    assert_eq!(
        queued.lowlevel_param("KUmsZeitCamt1.allaccounts"),
        Some("N")
    );
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
        tan_media_names: vec!["phone".to_owned(), "app".to_owned()],
        tan_segment_version: Some("5".to_owned()),
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
        bpd_parameters: BTreeMap::from([(
            "Params.TAN2StepPar5.ParTAN2Step.orderhashmode".to_owned(),
            "2".to_owned(),
        )]),
        twostep_mechanisms: BTreeMap::new(),
        allowed_twostep_mechanisms: vec!["921".to_owned(), "922".to_owned()],
        persistent_data: BTreeMap::from([(
            "dauer_ORDER123".to_owned(),
            BTreeMap::from([("DauerDetails.firstdate".to_owned(), "2025-11-01".to_owned())]),
        )]),
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
fn passport_resolves_orderhash_mode_from_bpd_like_hbci4java_query() {
    let passport = PinTanPassport::new(PinTanPassportData {
        tan_segment_version: Some("5".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params_1.TAN2StepPar4.ParTAN2Step.orderhashmode".to_owned(),
                "1".to_owned(),
            ),
            (
                "Params_2.TAN2StepPar5.ParTAN2Step.orderhashmode".to_owned(),
                "2".to_owned(),
            ),
            (
                "Params_2.TAN2StepPar5.ParTAN2Step.needorderaccount".to_owned(),
                "2".to_owned(),
            ),
        ]),
        ..PinTanPassportData::default()
    });

    assert_eq!(passport.order_hash_mode_code().as_deref(), Some("2"));
    assert_eq!(
        passport.tan2step_parameter("needorderaccount").as_deref(),
        Some("2")
    );
    let secmech = passport.current_secmech_info();
    assert_eq!(secmech.get("segversion").map(String::as_str), Some("5"));
    assert_eq!(secmech.get("orderhashmode").map(String::as_str), Some("2"));
}

#[test]
fn passport_extracts_twostep_mechanisms_from_bpd_like_hbci4java_set_bpd() {
    let mut passport = PinTanPassport::new(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        ..PinTanPassportData::default()
    });
    let updated = passport.update_parameter_data_from_values(
        &BTreeMap::from([
            (
                "DialogInitRes.BPD.Params_1.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "DialogInitRes.BPD.Params_1.TAN2StepPar5.ParTAN2Step.name".to_owned(),
                "photoTAN new".to_owned(),
            ),
            (
                "DialogInitRes.BPD.Params_1.TAN2StepPar5.ParTAN2Step.orderhashmode".to_owned(),
                "2".to_owned(),
            ),
            (
                "DialogInitRes.BPD.Params_1.TAN2StepPar5.ParTAN2Step.needorderaccount".to_owned(),
                "2".to_owned(),
            ),
            (
                "DialogInitRes.BPD.Params_1.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "1".to_owned(),
            ),
            (
                "DialogInitRes.BPD.Params_2.TAN2StepPar4.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "DialogInitRes.BPD.Params_2.TAN2StepPar4.ParTAN2Step.name".to_owned(),
                "chipTAN old".to_owned(),
            ),
            (
                "DialogInitRes.BPD.Params_2.TAN2StepPar4.ParTAN2Step.orderhashmode".to_owned(),
                "1".to_owned(),
            ),
            (
                "DialogInitRes.BPD.Params_3.TAN2StepPar4.ParTAN2Step.secfunc".to_owned(),
                "922".to_owned(),
            ),
            (
                "DialogInitRes.BPD.Params_3.TAN2StepPar4.ParTAN2Step.name".to_owned(),
                "mobileTAN".to_owned(),
            ),
        ]),
        "DialogInitRes",
    );

    assert_eq!(updated, 1);
    assert_eq!(passport.twostep_mechanisms().len(), 2);
    let mechanism = passport
        .twostep_mechanisms()
        .get("921")
        .expect("selected mechanism");
    assert_eq!(mechanism.get("segversion").map(String::as_str), Some("5"));
    assert_eq!(mechanism.get("secfunc").map(String::as_str), Some("921"));
    assert_eq!(
        mechanism.get("name").map(String::as_str),
        Some("photoTAN new")
    );
    assert_eq!(
        mechanism.get("orderhashmode").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        mechanism.get("needorderaccount").map(String::as_str),
        Some("2")
    );
    assert_eq!(passport.tan_segment_version(), "5");
    assert_eq!(passport.order_hash_mode_code().as_deref(), Some("2"));
    assert_eq!(
        passport.tan2step_parameter("needorderaccount").as_deref(),
        Some("2")
    );
    let current = passport.current_secmech_info();
    assert_eq!(current.get("secfunc").map(String::as_str), Some("921"));
    assert_eq!(current.get("process").map(String::as_str), Some("1"));

    let other = passport
        .twostep_mechanisms()
        .get("922")
        .expect("other mechanism");
    assert_eq!(other.get("segversion").map(String::as_str), Some("4"));
    assert_eq!(other.get("name").map(String::as_str), Some("mobileTAN"));
}

#[test]
fn passport_imports_allowed_twostep_mechanisms_from_3920_return_values() {
    let mut passport = PinTanPassport::new(PinTanPassportData {
        allowed_twostep_mechanisms: vec!["900".to_owned()],
        ..PinTanPassportData::default()
    });
    let mut first = HbciReturnValue::new("3920", "Zugelassene TAN-Verfahren");
    first.params = vec!["921".to_owned(), "922".to_owned(), "921".to_owned()];
    let mut second = HbciReturnValue::new("3920", "Weitere TAN-Verfahren");
    second.params = vec!["923".to_owned(), String::new()];
    let mut ignored = HbciReturnValue::new("0010", "OK");
    ignored.params = vec!["999".to_owned()];
    let status = HbciMsgStatus::from_statuses(
        HbciStatus::from_return_values([first, ignored]),
        HbciStatus::from_return_values([second]),
    );

    let updated = passport.update_allowed_twostep_mechanisms_from_status(&status);

    assert_eq!(updated, 3);
    assert_eq!(passport.allowed_twostep_mechanisms(), ["921", "922", "923"]);
    assert_eq!(
        passport.update_allowed_twostep_mechanisms_from_status(&status),
        0
    );

    let empty_status = HbciMsgStatus::from_statuses(
        HbciStatus::from_return_values([HbciReturnValue::new("3920", "Keine Parameter")]),
        HbciStatus::default(),
    );
    assert_eq!(
        passport.update_allowed_twostep_mechanisms_from_status(&empty_status),
        0
    );
    assert_eq!(passport.allowed_twostep_mechanisms(), ["921", "922", "923"]);
}

#[test]
fn passport_imports_tan_media_names_from_upd_values() {
    let mut passport = PinTanPassport::new(PinTanPassportData::default());

    let count = passport.update_parameter_data_from_values(
        &BTreeMap::from([(
            "DialogInitRes.UPD.tanmedia.names".to_owned(),
            "mobile|push||photo".to_owned(),
        )]),
        "DialogInitRes",
    );

    assert_eq!(count, 1);
    assert_eq!(passport.tan_media_names(), ["mobile", "push", "photo"]);
}

#[test]
fn passport_selects_single_user_allowed_twostep_method() {
    let mut passport = PinTanPassport::new(PinTanPassportData {
        bpd_parameters: pintan_bpd("N", &[("921", "photoTAN"), ("922", "pushTAN")]),
        allowed_twostep_mechanisms: vec!["922".to_owned()],
        ..PinTanPassportData::default()
    });

    assert_eq!(passport.current_tan_method(), None);

    let selection = passport.determine_tan_method();

    assert_eq!(selection, TanMethodSelection::Selected("922".to_owned()));
    assert_eq!(passport.current_tan_method(), Some("922"));
}

#[test]
fn passport_reuses_current_user_allowed_twostep_method() {
    let mut passport = PinTanPassport::new(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        bpd_parameters: pintan_bpd("N", &[("921", "photoTAN"), ("922", "pushTAN")]),
        allowed_twostep_mechanisms: vec!["921".to_owned(), "922".to_owned()],
        ..PinTanPassportData::default()
    });

    let selection = passport.determine_tan_method();

    assert_eq!(selection, TanMethodSelection::Selected("921".to_owned()));
    assert_eq!(passport.current_tan_method(), Some("921"));
}

#[test]
fn passport_reports_user_selection_when_multiple_user_methods_are_possible() {
    let mut passport = PinTanPassport::new(PinTanPassportData {
        bpd_parameters: pintan_bpd("N", &[("922", "pushTAN"), ("921", "photoTAN")]),
        allowed_twostep_mechanisms: vec!["921".to_owned(), "922".to_owned()],
        ..PinTanPassportData::default()
    });

    let selection = passport.determine_tan_method();

    let TanMethodSelection::NeedsUserSelection(options) = selection else {
        panic!("expected user selection");
    };
    assert_eq!(
        options
            .iter()
            .map(|option| (option.id.as_str(), option.name.as_deref()))
            .collect::<Vec<_>>(),
        [("921", Some("photoTAN")), ("922", Some("pushTAN")),]
    );
    assert_eq!(passport.current_tan_method(), None);
}

#[test]
fn passport_one_step_fallback_does_not_persist_tan_method() {
    let mut passport = PinTanPassport::new(PinTanPassportData {
        bpd_parameters: pintan_bpd("J", &[("921", "photoTAN")]),
        ..PinTanPassportData::default()
    });

    let selection = passport.determine_tan_method();

    assert_eq!(selection, TanMethodSelection::OneStepFallback);
    assert_eq!(passport.current_tan_method(), None);
}

#[test]
fn passport_reports_pintan_segment_tan_info_like_original_bpd_lookup() {
    let passport = PinTanPassport::new(PinTanPassportData {
        bpd_parameters: BTreeMap::from([
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.segcode".to_owned(),
                "HKSAL".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.needtan".to_owned(),
                "N".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV2.segcode".to_owned(),
                "HKKAZ".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV2.needtan".to_owned(),
                "J".to_owned(),
            ),
            (
                "Params.SaldoPar7.SegHead.code".to_owned(),
                "HISALS".to_owned(),
            ),
            (
                "Params.KUmsZeitPar7.SegHead.code".to_owned(),
                "HIKAZS".to_owned(),
            ),
            (
                "Params.ExamplePar1.SegHead.code".to_owned(),
                "HIXYZS".to_owned(),
            ),
        ]),
        ..PinTanPassportData::default()
    });

    assert_eq!(
        passport.pin_tan_info_for_segment_code("HKSAL").as_deref(),
        Some("N")
    );
    assert_eq!(
        passport.pin_tan_info_for_segment_code("HKKAZ").as_deref(),
        Some("J")
    );
    assert_eq!(passport.pin_tan_info_for_segment_code("HKXYZ"), None);
    assert_eq!(
        passport.pin_tan_info_for_segment_code("HNHBK").as_deref(),
        Some("A")
    );
}

#[test]
fn passport_returns_no_pintan_segment_tan_info_without_bpd() {
    let passport = PinTanPassport::new(PinTanPassportData::default());

    assert_eq!(passport.pin_tan_info_for_segment_code("HNHBK"), None);
}

#[test]
fn passport_reports_bank_selection_when_no_user_methods_and_one_step_disallowed() {
    let mut passport = PinTanPassport::new(PinTanPassportData {
        bpd_parameters: pintan_bpd("N", &[("922", "pushTAN"), ("921", "photoTAN")]),
        ..PinTanPassportData::default()
    });

    let selection = passport.determine_tan_method();

    let TanMethodSelection::NeedsUserSelection(options) = selection else {
        panic!("expected user selection");
    };
    assert_eq!(
        options
            .iter()
            .map(|option| (option.id.as_str(), option.name.as_deref()))
            .collect::<Vec<_>>(),
        [("921", Some("photoTAN")), ("922", Some("pushTAN")),]
    );
    assert_eq!(passport.current_tan_method(), None);
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

    assert_eq!(count, 10);
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
    assert_eq!(
        passport
            .bpd_parameters()
            .get("BPA.KIK.blz")
            .map(String::as_str),
        Some("12345678")
    );
}

#[tokio::test]
async fn handler_init_imports_upd_accounts_from_replay_response() {
    let passport = passport_with_cached_pin(PinTanPassportData {
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
    assert_signed_dialog_init_request(&body, "customer", "0", "0");
}

#[tokio::test]
async fn handler_init_imports_allowed_twostep_mechanisms_from_3920_response() {
    let passport = passport_with_cached_pin(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+3920::Zugelassene TAN-Verfahren:922:921:922+0010::Initialisiert",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);

    handler.init().await.expect("dialog init replay response");

    assert_eq!(
        handler.passport().allowed_twostep_mechanisms(),
        ["921", "922"]
    );
}

#[tokio::test]
async fn handler_init_requests_pin_for_signed_dialog_init() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(ScriptedCallback {
            events: events.clone(),
            responses: Arc::new(Mutex::new(VecDeque::from([CallbackResponse::value(
                "12345",
            )]))),
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
    let replay =
        ReplayCommClient::new([Ok(custom_msg_response(&["HIRMG:2:2+0010::Initialisiert"]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());

    handler.init().await.expect("dialog init replay response");

    assert_eq!(handler.passport().pin(), Some("12345"));
    let requests = replay.requests().expect("requests");
    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert_signed_dialog_init_request(&body, "customer", "0", "0");

    let reasons = events
        .lock()
        .expect("callback event lock")
        .iter()
        .map(|event| event.reason)
        .collect::<Vec<_>>();
    assert_eq!(
        reasons,
        [
            CallbackReason::NeedPtPin,
            CallbackReason::NeedConnection,
            CallbackReason::CloseConnection,
        ]
    );
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_init_selects_single_allowed_twostep_method_from_cached_bpd() {
    let passport = passport_with_cached_pin(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        bpd_parameters: pintan_bpd("N", &[("921", "photoTAN"), ("922", "pushTAN")]),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+3920::Zugelassene TAN-Verfahren:922+0010::Initialisiert",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);

    handler.init().await.expect("dialog init replay response");

    assert_eq!(handler.passport().current_tan_method(), Some("922"));
}

#[tokio::test]
async fn handler_init_asks_callback_for_ambiguous_twostep_method() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(SecmechSelectingCallback {
            events: events.clone(),
            selection: "921".to_owned(),
        }),
    )
    .expect("runtime init");

    let passport = passport_with_cached_pin(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        bpd_parameters: pintan_bpd("N", &[("921", "photoTAN"), ("922", "pushTAN")]),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+3920::Zugelassene TAN-Verfahren:922:921+0010::Initialisiert",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);

    handler.init().await.expect("dialog init replay response");

    assert_eq!(handler.passport().current_tan_method(), Some("921"));
    let events = events.lock().expect("callback event lock");
    let secmech_event = events
        .iter()
        .find(|event| event.reason == CallbackReason::NeedPtSecMech)
        .expect("secmech callback event");
    assert_eq!(secmech_event.data_type, CallbackDataType::Select);
    assert_eq!(
        secmech_event.message,
        "*** Select a pintan method from the list"
    );
    assert_eq!(
        secmech_event.current_value.as_deref(),
        Some("921:photoTAN|922:pushTAN")
    );
    drop(events);
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_init_rejects_unsupported_callback_twostep_method() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(SecmechSelectingCallback {
            events: events.clone(),
            selection: "999".to_owned(),
        }),
    )
    .expect("runtime init");

    let passport = passport_with_cached_pin(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        bpd_parameters: pintan_bpd("N", &[("921", "photoTAN"), ("922", "pushTAN")]),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+3920::Zugelassene TAN-Verfahren:922:921+0010::Initialisiert",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);

    let err = handler
        .init()
        .await
        .expect_err("unsupported callback method is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::Callback);
    assert_eq!(err.message(), "selected pintan method not supported: 999");
    assert_eq!(handler.passport().current_tan_method(), None);
    drop(events);
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_init_uses_cached_bpd_and_upd_versions() {
    let passport = passport_with_cached_pin(PinTanPassportData {
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
    assert_signed_dialog_init_request(&body, "customer", "5", "7");
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

    let passport = passport_with_cached_pin(PinTanPassportData {
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
    let passport = passport_with_cached_pin(PinTanPassportData {
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
    let passport = passport_with_cached_pin(PinTanPassportData {
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
    assert_signed_dialog_init_request(&init_body, "customer", "0", "0");

    let execute_body = String::from_utf8(requests[1].body.clone()).expect("request body is text");
    assert_signed_custom_msg_request(&execute_body, "DIALOG1", "2", 5);
    assert!(execute_body.contains("HKSAL:3:7+DE02123456780000000000::0001234567::280:12345678+N'"));
}

#[tokio::test]
async fn handler_execute_rejects_mismatched_response_reference() {
    let passport = passport_with_cached_pin(PinTanPassportData {
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
    let passport = passport_with_cached_pin(PinTanPassportData {
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
    let passport = passport_with_cached_pin(PinTanPassportData {
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
    assert_signed_dialog_end_request(&close_body, "DIALOG1", "3");
}

#[tokio::test]
async fn handler_close_preserves_context_on_dialog_end_error() {
    let passport = passport_with_cached_pin(PinTanPassportData {
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
    assert_signed_dialog_end_request(&close_body, "DIALOG1", "2");
}

#[tokio::test]
async fn handler_close_accepts_segment_error_when_global_status_is_ok_like_original() {
    let passport = passport_with_cached_pin(PinTanPassportData {
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
                "HIRMS:3:2+9010:3:Segmentfehler",
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
    let passport = passport_with_cached_pin(PinTanPassportData {
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
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HIRMS:3:2+0020:3:Saldo bereitgestellt",
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
    assert_eq!(status.job_results[0].seg_num(), Some("3"));
    assert_eq!(
        status.job_results[0].job_id_for_date("20260606"),
        "20260606/0/1/3"
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
            "0020:Saldo bereitgestellt (3)".to_owned()
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
    assert_signed_custom_msg_request(&body, "0", "1", 5);
    assert!(body.contains("HKSAL:3:7+DE02123456780000000000+N'"));
    assert!(!body.contains("SaldoReq"));
}

#[tokio::test]
async fn handler_renders_and_collects_acc_info_like_original() {
    let passport = passport_with_cached_pin(PinTanPassportData {
        accounts: vec![Konto {
            country: Some("DE".to_owned()),
            blz: Some("12345678".to_owned()),
            number: Some("0000000000".to_owned()),
            subnumber: Some("".to_owned()),
            curr: Some("EUR".to_owned()),
            ..Konto::default()
        }],
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HIKIF:3:2+0000000000::280:12345678+1+Max+Mustermann+Girokonto+EUR+20200102+1,234+0,500+12,345+1000,00:EUR+1111111111::280:12345678+Max Mustermann::Street 1:12345:Berlin:280:49123::mail.example.test+4+3+Bemerkung",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let job = handler.new_job("AccInfo").expect("job is in registry");

    handler.add_to_queue(job);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "AccInfo");
    assert!(status.job_results[0].success);
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.My.number")
            .map(String::as_str),
        Some("0000000000")
    );
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.kredit.value")
            .map(String::as_str),
        Some("1000.00")
    );

    let Some(HbciJobResultData::AccInfo(result)) = status.job_results[0].result.as_ref() else {
        panic!("expected AccInfo result data");
    };
    assert_eq!(result.entries.len(), 1);
    let entry = &result.entries[0];
    assert_eq!(entry.account.number.as_deref(), Some("0000000000"));
    assert_eq!(entry.account.blz.as_deref(), Some("12345678"));
    assert_eq!(entry.account.country.as_deref(), Some("DE"));
    assert_eq!(entry.account.name.as_deref(), Some("Max"));
    assert_eq!(entry.account.name2.as_deref(), Some("Mustermann"));
    assert_eq!(entry.account.account_type.as_deref(), Some("Girokonto"));
    assert_eq!(entry.account.curr.as_deref(), Some("EUR"));
    assert_eq!(entry.account_kind, Some(1));
    assert_eq!(entry.created.as_deref(), Some("2020-01-02"));
    assert_eq!(entry.sollzins.as_deref(), Some("1.234"));
    assert_eq!(entry.habenzins.as_deref(), Some("0.500"));
    assert_eq!(entry.ueberzins.as_deref(), Some("12.345"));
    assert_eq!(
        entry.kredit.as_ref().map(|value| value.value.as_str()),
        Some("1000.00")
    );
    assert_eq!(
        entry
            .ref_account
            .as_ref()
            .and_then(|account| account.number.as_deref()),
        Some("1111111111")
    );
    assert_eq!(entry.versandart, Some(4));
    assert_eq!(entry.turnus, Some(3));
    assert_eq!(entry.comment.as_deref(), Some("Bemerkung"));
    let address = entry.address.as_ref().expect("address");
    assert_eq!(address.name1.as_deref(), Some("Max Mustermann"));
    assert_eq!(address.street_pf.as_deref(), Some("Street 1"));
    assert_eq!(address.plz.as_deref(), Some("12345"));
    assert_eq!(address.ort.as_deref(), Some("Berlin"));
    assert_eq!(address.country.as_deref(), Some("DE"));
    assert_eq!(address.tel.as_deref(), Some("49123"));
    assert_eq!(address.email.as_deref(), Some("mail.example.test"));

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 1);

    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert_signed_custom_msg_request(&body, "0", "1", 5);
    assert!(
        body.contains("HKKIF:3:2+0000000000::280:12345678+N'"),
        "{body}"
    );
}

#[tokio::test]
async fn handler_renders_and_collects_dauer_sepa_list_envelope_like_original() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let pain = concat!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>",
        "<Document xmlns=\"urn:sepade:xsd:pain.001.001.02\">",
        "<pain.001.001.02>",
        "<GrpHdr><InitgPty><Nm>Maxine Mustermann</Nm></InitgPty></GrpHdr>",
        "<PmtInf>",
        "<PmtInfId>PMT-ORDER123</PmtInfId>",
        "<ReqdExctnDt>2026-01-02</ReqdExctnDt>",
        "<DbtrAcct><Id><IBAN>DE02123456780000000000</IBAN></Id></DbtrAcct>",
        "<DbtrAgt><FinInstnId><BIC>MARKDEF1100</BIC></FinInstnId></DbtrAgt>",
        "<CdtTrfTxInf>",
        "<PmtId><EndToEndId>E2E-ORDER123</EndToEndId></PmtId>",
        "<Amt><InstdAmt Ccy=\"EUR\">12.30</InstdAmt></Amt>",
        "<CdtrAgt><FinInstnId><BIC>DEUTDEDB277</BIC></FinInstnId></CdtrAgt>",
        "<Cdtr><Nm>Receiver Name</Nm></Cdtr>",
        "<CdtrAcct><Id><IBAN>DE99123456780000000000</IBAN></Id></CdtrAcct>",
        "<RmtInf><Ustrd>Standing Usage</Ustrd></RmtInf>",
        "</CdtTrfTxInf>",
        "</PmtInf>",
        "</pain.001.001.02>",
        "</Document>",
    );
    let hicdb = format!(
        "HICDB:3:2+DE02123456780000000000:MARKDEF1100+urn?:sepade?:xsd?:pain.001.001.02+@{}@{}+ORDER123+20251101:M:1:1::20261231+J:20260101:20260201:2:2,00:EUR+N+J+N",
        pain.len(),
        pain
    );
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        hicdb.as_str(),
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut job = handler
        .new_job("DauerSEPAList")
        .expect("job is in registry");
    job.try_set_param("src.iban", "DE02123456780000000000")
        .expect("source iban alias is accepted");
    job.try_set_param("src.bic", "MARKDEF1100")
        .expect("source bic alias is accepted");
    job.try_set_param_int("maxentries", 10)
        .expect("max entries is accepted");

    handler.add_to_queue(job);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "DauerSEPAList");
    assert!(status.job_results[0].success);
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.DauerDetails.firstdate")
            .map(String::as_str),
        Some("2025-11-01")
    );
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.sepapain")
            .map(String::as_str),
        Some(pain)
    );

    let Some(HbciJobResultData::DauerList(result)) = status.job_results[0].result.as_ref() else {
        panic!("expected DauerList result data");
    };
    assert_eq!(result.entries.len(), 1);
    let entry = &result.entries[0];
    assert_eq!(entry.my.iban.as_deref(), Some("DE02123456780000000000"));
    assert_eq!(entry.my.bic.as_deref(), Some("MARKDEF1100"));
    assert_eq!(entry.other.name.as_deref(), Some("Receiver Name"));
    assert_eq!(entry.other.iban.as_deref(), Some("DE99123456780000000000"));
    assert_eq!(entry.other.bic.as_deref(), Some("DEUTDEDB277"));
    assert_eq!(
        entry.value.as_ref().map(|value| value.value.as_str()),
        Some("12.30")
    );
    assert_eq!(
        entry.value.as_ref().and_then(|value| value.curr.as_deref()),
        Some("EUR")
    );
    assert_eq!(entry.usage, ["Standing Usage".to_owned()]);
    assert_eq!(entry.pmtinfid.as_deref(), Some("PMT-ORDER123"));
    assert_eq!(entry.sepadescr.as_deref(), Some(PAIN_001_001_02_URN));
    assert_eq!(entry.sepapain_raw.as_deref(), Some(pain));
    assert_eq!(entry.orderid.as_deref(), Some("ORDER123"));
    assert_eq!(entry.firstdate.as_deref(), Some("2025-11-01"));
    assert_eq!(entry.timeunit.as_deref(), Some("M"));
    assert_eq!(entry.turnus, Some(1));
    assert_eq!(entry.execday, Some(1));
    assert_eq!(entry.lastdate.as_deref(), Some("2026-12-31"));
    assert!(!entry.can_change);
    assert!(entry.can_skip);
    assert!(!entry.can_delete);
    let aussetzung = entry.aussetzung.as_ref().expect("aussetzung");
    assert!(aussetzung.annual);
    assert_eq!(aussetzung.startdate.as_deref(), Some("2026-01-01"));
    assert_eq!(aussetzung.enddate.as_deref(), Some("2026-02-01"));
    assert_eq!(aussetzung.number.as_deref(), Some("2"));
    assert_eq!(
        aussetzung
            .newvalue
            .as_ref()
            .map(|value| value.value.as_str()),
        Some("2.00")
    );
    let snapshot = handler
        .passport()
        .get_persistent_data("dauer_ORDER123")
        .expect("dauer persistent data");
    assert_eq!(
        snapshot.get("DauerDetails.firstdate").map(String::as_str),
        Some("2025-11-01")
    );
    assert_eq!(snapshot.get("sepapain").map(String::as_str), Some(pain));
    assert_eq!(
        snapshot
            .get("Aussetzung.newvalue.value")
            .map(String::as_str),
        Some("2.00")
    );
    assert!(!snapshot.contains_key("orderid"));
    assert!(!snapshot.keys().any(|key| key.starts_with("SegHead.")));

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 1);

    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert_signed_custom_msg_request(&body, "0", "1", 5);
    assert!(
        body.contains(
            "HKCDB:3:2+DE02123456780000000000:MARKDEF1100+urn?:sepade?:xsd?:pain.001.001.02++10'"
        ),
        "{body}"
    );
}

#[tokio::test]
async fn handler_renders_and_collects_dauer_sepa_new_like_original() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HICDE:3:1+ORDERNEW",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut job = handler.new_job("DauerSEPANew").expect("job is in registry");
    job.try_set_param("src.iban", "DE02123456780000000000")
        .expect("source iban is accepted");
    job.try_set_param("src.bic", "MARKDEF1100")
        .expect("source bic is accepted");
    job.try_set_param("_sepapain", "<Document/>")
        .expect("raw pain is accepted for first generator-free slice");
    job.try_set_param_date("firstdate", "2025-11-01")
        .expect("firstdate is accepted");
    job.try_set_param("timeunit", "M")
        .expect("timeunit is accepted");
    job.try_set_param_int("turnus", 1)
        .expect("turnus is accepted");
    job.try_set_param_int("execday", 1)
        .expect("execday is accepted");
    job.try_set_param_date("lastdate", "2026-12-31")
        .expect("lastdate is accepted");

    handler.try_add_to_queue(job).expect("constraints resolve");
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "DauerSEPANew");
    assert!(status.job_results[0].success);
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.orderid")
            .map(String::as_str),
        Some("ORDERNEW")
    );
    let Some(HbciJobResultData::DauerNew(result)) = status.job_results[0].result.as_ref() else {
        panic!("expected DauerNew result data");
    };
    assert_eq!(result.order_id.as_deref(), Some("ORDERNEW"));

    let snapshot = handler
        .passport()
        .get_persistent_data("dauer_ORDERNEW")
        .expect("dauer new persistent data");
    assert_eq!(
        snapshot.get("My.iban").map(String::as_str),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        snapshot.get("My.bic").map(String::as_str),
        Some("MARKDEF1100")
    );
    assert_eq!(
        snapshot.get("sepadescr").map(String::as_str),
        Some(PAIN_001_001_02_URN)
    );
    assert_eq!(
        snapshot.get("sepapain").map(String::as_str),
        Some("B<Document/>")
    );
    assert_eq!(
        snapshot.get("DauerDetails.firstdate").map(String::as_str),
        Some("2025-11-01")
    );
    assert_eq!(
        snapshot.get("DauerDetails.lastdate").map(String::as_str),
        Some("2026-12-31")
    );
    assert!(!snapshot.keys().any(|key| key.starts_with("sepa.")));

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 1);

    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert_signed_custom_msg_request(&body, "0", "1", 5);
    assert!(
        body.contains(
            "HKCDE:3:1+DE02123456780000000000:MARKDEF1100+urn?:sepade?:xsd?:pain.001.001.02+@11@<Document/>+20251101:M:1:1:20261231'"
        ),
        "{body}"
    );
}

#[tokio::test]
async fn handler_generates_pain_for_dauer_sepa_new_from_original_params() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HICDE:3:1+ORDERGEN",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut job = handler.new_job("DauerSEPANew").expect("job is in registry");
    job.try_set_param("src.iban", "DE02123456780000000000")
        .expect("source iban is accepted");
    job.try_set_param("src.bic", "MARKDEF1100")
        .expect("source bic is accepted");
    job.try_set_param("src.name", "Sender Name")
        .expect("source name is accepted");
    job.try_set_param("dst.name", "Receiver Name")
        .expect("destination name is accepted");
    job.try_set_param("dst.iban", "DE99123456780000000000")
        .expect("destination iban is accepted");
    job.try_set_param("dst.bic", "DEUTDEDB277")
        .expect("destination bic is accepted");
    job.try_set_param("btg.value", "12.30")
        .expect("amount value is accepted");
    job.try_set_param("usage", "Standing & generated")
        .expect("usage is accepted");
    job.try_set_param("sepaid", "SEPA-GEN")
        .expect("sepa id is accepted");
    job.try_set_param_date("firstdate", "2025-11-01")
        .expect("firstdate is accepted");
    job.try_set_param("timeunit", "M")
        .expect("timeunit is accepted");
    job.try_set_param_int("turnus", 1)
        .expect("turnus is accepted");
    job.try_set_param_int("execday", 1)
        .expect("execday is accepted");
    job.try_set_param_date("lastdate", "2026-12-31")
        .expect("lastdate is accepted");

    handler.try_add_to_queue(job).expect("constraints resolve");
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "DauerSEPANew");
    let Some(HbciJobResultData::DauerNew(result)) = status.job_results[0].result.as_ref() else {
        panic!("expected DauerNew result data");
    };
    assert_eq!(result.order_id.as_deref(), Some("ORDERGEN"));

    let snapshot = handler
        .passport()
        .get_persistent_data("dauer_ORDERGEN")
        .expect("dauer new persistent data");
    let generated_pain = snapshot
        .get("sepapain")
        .expect("generated pain is persisted");
    assert!(generated_pain.starts_with("B<?xml"), "{generated_pain}");
    assert!(generated_pain.contains("<MsgId>SEPA-GEN</MsgId>"));
    assert!(generated_pain.contains("<PmtInfId>SEPA-GEN</PmtInfId>"));
    assert!(generated_pain.contains("<ReqdExctnDt>1999-01-01</ReqdExctnDt>"));
    assert!(generated_pain.contains("<EndToEndId>NOTPROVIDED</EndToEndId>"));
    assert!(generated_pain.contains("<Ustrd>Standing &amp; generated</Ustrd>"));

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 1);

    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert_signed_custom_msg_request(&body, "0", "1", 5);
    assert!(body.contains("HKCDE:3:1+DE02123456780000000000:MARKDEF1100+"));
    assert!(body.contains("<MsgId>SEPA-GEN</MsgId>"), "{body}");
    assert!(
        body.contains("<DbtrAcct><Id><IBAN>DE02123456780000000000</IBAN></Id></DbtrAcct>"),
        "{body}"
    );
    assert!(
        body.contains("<CdtrAcct><Id><IBAN>DE99123456780000000000</IBAN></Id></CdtrAcct>"),
        "{body}"
    );
}

#[tokio::test]
async fn handler_renders_and_collects_dauer_sepa_edit_like_original() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HICDN:3:1+ORDERNEW+OLDORDER",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut job = handler
        .new_job("DauerSEPAEdit")
        .expect("job is in registry");
    job.try_set_param("src.iban", "DE02123456780000000000")
        .expect("source iban is accepted");
    job.try_set_param("src.bic", "MARKDEF1100")
        .expect("source bic is accepted");
    job.try_set_param("_sepapain", "<Document/>")
        .expect("raw pain is accepted for first generator-free slice");
    job.try_set_param("orderid", "OLDORDER")
        .expect("orderid is accepted");
    job.try_set_param_date("date", "2025-10-15")
        .expect("date is accepted");
    job.try_set_param_date("firstdate", "2025-11-01")
        .expect("firstdate is accepted");
    job.try_set_param("timeunit", "M")
        .expect("timeunit is accepted");
    job.try_set_param_int("turnus", 1)
        .expect("turnus is accepted");
    job.try_set_param_int("execday", 1)
        .expect("execday is accepted");
    job.try_set_param_date("lastdate", "2026-12-31")
        .expect("lastdate is accepted");

    handler.try_add_to_queue(job).expect("constraints resolve");
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "DauerSEPAEdit");
    assert!(status.job_results[0].success);
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.orderid")
            .map(String::as_str),
        Some("ORDERNEW")
    );
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.orderidold")
            .map(String::as_str),
        Some("OLDORDER")
    );
    let Some(HbciJobResultData::DauerEdit(result)) = status.job_results[0].result.as_ref() else {
        panic!("expected DauerEdit result data");
    };
    assert_eq!(result.order_id.as_deref(), Some("ORDERNEW"));
    assert_eq!(result.order_id_old.as_deref(), Some("OLDORDER"));

    let snapshot = handler
        .passport()
        .get_persistent_data("dauer_ORDERNEW")
        .expect("dauer edit persistent data");
    assert_eq!(
        snapshot.get("orderid").map(String::as_str),
        Some("OLDORDER")
    );
    assert_eq!(snapshot.get("date").map(String::as_str), Some("2025-10-15"));
    assert_eq!(
        snapshot.get("My.iban").map(String::as_str),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        snapshot.get("sepapain").map(String::as_str),
        Some("B<Document/>")
    );
    assert_eq!(
        snapshot.get("DauerDetails.firstdate").map(String::as_str),
        Some("2025-11-01")
    );
    assert!(!snapshot.keys().any(|key| key.starts_with("sepa.")));

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 1);

    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert_signed_custom_msg_request(&body, "0", "1", 5);
    assert!(
        body.contains(
            "HKCDN:3:1+DE02123456780000000000:MARKDEF1100+urn?:sepade?:xsd?:pain.001.001.02+@11@<Document/>+20251015+OLDORDER+20251101:M:1:1:20261231'"
        ),
        "{body}"
    );
}

#[tokio::test]
async fn handler_renders_and_collects_dauer_sepa_del_like_original() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HICDN:3:1+ORDERDEL+OLDORDER",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut job = handler.new_job("DauerSEPADel").expect("job is in registry");
    job.try_set_param("src.iban", "DE02123456780000000000")
        .expect("source iban is accepted");
    job.try_set_param("src.bic", "MARKDEF1100")
        .expect("source bic is accepted");
    job.try_set_param("_sepapain", "<Document/>")
        .expect("raw pain is accepted for first generator-free slice");
    job.try_set_param("orderid", "OLDORDER")
        .expect("orderid is accepted");
    job.try_set_param_date("date", "2025-10-15")
        .expect("date is accepted");
    job.try_set_param_date("firstdate", "2025-11-01")
        .expect("firstdate is accepted");
    job.try_set_param("timeunit", "M")
        .expect("timeunit is accepted");
    job.try_set_param_int("turnus", 1)
        .expect("turnus is accepted");
    job.try_set_param_int("execday", 1)
        .expect("execday is accepted");
    job.try_set_param_date("lastdate", "2026-12-31")
        .expect("lastdate is accepted");

    handler.try_add_to_queue(job).expect("constraints resolve");
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "DauerSEPADel");
    assert!(status.job_results[0].success);
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.orderid")
            .map(String::as_str),
        Some("ORDERDEL")
    );
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.orderidold")
            .map(String::as_str),
        Some("OLDORDER")
    );
    let Some(HbciJobResultData::DauerEdit(result)) = status.job_results[0].result.as_ref() else {
        panic!("expected DauerEdit result data for delete job");
    };
    assert_eq!(result.order_id.as_deref(), Some("ORDERDEL"));
    assert_eq!(result.order_id_old.as_deref(), Some("OLDORDER"));

    let snapshot = handler
        .passport()
        .get_persistent_data("dauer_ORDERDEL")
        .expect("dauer del persistent data");
    assert_eq!(
        snapshot.get("orderid").map(String::as_str),
        Some("OLDORDER")
    );
    assert_eq!(snapshot.get("date").map(String::as_str), Some("2025-10-15"));
    assert_eq!(
        snapshot.get("My.iban").map(String::as_str),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        snapshot.get("sepapain").map(String::as_str),
        Some("B<Document/>")
    );
    assert!(!snapshot.keys().any(|key| key.starts_with("sepa.")));

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 1);

    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert_signed_custom_msg_request(&body, "0", "1", 5);
    assert!(
        body.contains(
            "HKCDL:3:1+DE02123456780000000000:MARKDEF1100+urn?:sepade?:xsd?:pain.001.001.02+@11@<Document/>+20251015+OLDORDER+20251101:M:1:1:20261231'"
        ),
        "{body}"
    );
}

#[tokio::test]
async fn handler_renders_and_collects_tan_media_list_like_original() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HITAB:3:4+2+M:1:::::::::::push-app+G:2:::::::::::inactive-card+S:1:::::::::::photo-app",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let job = handler.new_job("TANMediaList").expect("job is in registry");

    handler.add_to_queue(job);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "TANMediaList");
    assert!(status.job_results[0].success);
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.tanoption")
            .map(String::as_str),
        Some("2")
    );

    let Some(HbciJobResultData::TanMediaList(result)) = status.job_results[0].result.as_ref()
    else {
        panic!("expected TANMediaList result data");
    };
    assert_eq!(result.tan_option, Some(2));
    assert_eq!(result.media.len(), 3);
    assert_eq!(result.media[0].media_category.as_deref(), Some("M"));
    assert_eq!(result.media[0].status.as_deref(), Some("1"));
    assert_eq!(result.media[0].media_name.as_deref(), Some("push-app"));
    assert_eq!(result.media[1].media_name.as_deref(), Some("inactive-card"));
    assert_eq!(result.media[2].media_category.as_deref(), Some("S"));
    assert_eq!(result.media[2].media_name.as_deref(), Some("photo-app"));
    assert_eq!(
        handler.passport().tan_media_names(),
        &["push-app".to_owned(), "photo-app".to_owned()]
    );

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 1);

    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert_signed_custom_msg_request(&body, "0", "1", 5);
    assert!(body.contains("HKTAB:3:4+0+A'"), "{body}");
}

#[tokio::test]
async fn handler_renders_sepa_info_and_updates_passport_accounts_like_original() {
    let passport = passport_with_cached_pin(PinTanPassportData {
        accounts: vec![Konto {
            country: Some("DE".to_owned()),
            blz: Some("12345678".to_owned()),
            number: Some("0000000000".to_owned()),
            curr: Some("EUR".to_owned()),
            ..Konto::default()
        }],
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HISPA:3:1+J:DE02123456780000000000:MARKDEF1100:0000000000::280:12345678+N:DE99123456780000000000:MARKDEF1200:9999999999::280:12345678",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let job = handler.new_job("SEPAInfo").expect("job is in registry");

    handler.add_to_queue(job);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "SEPAInfo");
    assert!(status.job_results[0].success);
    assert!(status.job_results[0].result.is_none());
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.Acc.sepa")
            .map(String::as_str),
        Some("J")
    );
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.Acc.iban")
            .map(String::as_str),
        Some("DE02123456780000000000")
    );
    assert_eq!(
        status.job_results[0]
            .result_data
            .get("content.Acc_2.sepa")
            .map(String::as_str),
        Some("N")
    );

    let accounts = handler.passport().accounts();
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].iban.as_deref(), Some("DE02123456780000000000"));
    assert_eq!(accounts[0].bic.as_deref(), Some("MARKDEF1100"));

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 1);

    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert_signed_custom_msg_request(&body, "0", "1", 5);
    assert!(body.contains("HKSPA:3:1'"), "{body}");
}

#[tokio::test]
async fn handler_renders_saldo_request_from_lowlevel_params_like_original() {
    let passport = passport_with_cached_pin(signed_pintan_data());
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

    assert_signed_custom_msg_request(&body, "0", "1", 5);
    assert!(body.contains("HKSAL:3:7+DE02123456780000000000:MARKDEF1100+N+7'"));
}

#[tokio::test]
async fn handler_renders_tan2step5_with_applied_challenge_params_like_original() {
    const ORDER_SEGMENT: &str = "HKAOM:3:5+9876543210+100,99'";

    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_ok_response())]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let challenge_info = ChallengeInfo::parse_xml(CHALLENGE_DATA).expect("challenge data parses");
    let applied = challenge_info
        .apply_params(
            "HKAOM",
            &BTreeMap::from([
                ("BTG.value".to_owned(), "100.99".to_owned()),
                ("Other.number".to_owned(), "9876543210".to_owned()),
            ]),
            &BTreeMap::from([("id".to_owned(), "HHD1.4".to_owned())]),
        )
        .expect("challenge params apply")
        .expect("known challenge data");

    let mut hktan = handler.new_job("TAN2Step").expect("job is in registry");
    hktan.try_set_param("process", "1").expect("process");
    hktan
        .try_set_param("ordersegcode", "HKAOM")
        .expect("order segment code");
    hktan
        .try_set_param("orderaccount.number", "12345678")
        .expect("order account number");
    hktan
        .try_set_param("orderaccount.blz", "12345678")
        .expect("order account bank code");
    let raw_orderhash = OrderHashMode::Sha1
        .hash_segment(ORDER_SEGMENT)
        .expect("order segment hashes");
    hktan
        .try_set_param("orderhash", raw_orderhash)
        .expect("order hash");
    hktan
        .try_set_param("notlasttan", "N")
        .expect("not last TAN");
    for (key, value) in applied.to_hktan_params() {
        hktan
            .try_set_param(key, value)
            .expect("applied challenge parameter is accepted");
    }

    handler
        .try_add_to_queue(hktan)
        .expect("TAN2Step verifies and queues");
    let status = handler.execute().await.expect("replay response");
    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "TAN2Step");

    let requests = replay.requests().expect("requests");
    let body = &requests[0].body;
    let hash_prefix = b"HKTAN:3:5+1+HKAOM+::12345678::280:12345678+@20@";
    let hash_start = find_bytes(body, hash_prefix).expect("HKTAN hash prefix");
    let payload_start = hash_start + hash_prefix.len();
    let payload_end = payload_start + 20;
    assert_eq!(
        &body[payload_start..payload_end],
        OrderHashMode::Sha1
            .hash_segment_bytes(ORDER_SEGMENT)
            .expect("expected hash")
    );
    assert!(
        find_bytes(body, b"+++N+++10+100,99:::9876543210'HNSHA:4:2+").is_some(),
        "{}",
        String::from_utf8_lossy(body)
    );
    assert_signed_custom_msg_request_bytes(body, "0", "1", 5);
}

#[tokio::test]
async fn handler_prepares_process1_hktan_orderhash_from_rendered_task_segment() {
    const ORDER_SEGMENT: &str = "HKSAL:3:7+DE02123456780000000000+N'";

    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        tan_media: Some("sms-name".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.orderhashmode".to_owned(),
                "2".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.needorderaccount".to_owned(),
                "2".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_ok_response())]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo
        .try_set_param("my.iban", "DE02123456780000000000")
        .expect("saldo account");

    let hktan = handler
        .new_tan2step_process1_job(&saldo, None)
        .expect("process-1 HKTAN prepares");
    let expected_hash = OrderHashMode::Sha1
        .hash_segment(ORDER_SEGMENT)
        .expect("order segment hashes");
    let expected_lowlevel_hash = format!("B{expected_hash}");
    assert_eq!(hktan.param("process"), Some("1"));
    assert_eq!(hktan.param("ordersegcode"), Some("HKSAL"));
    assert_eq!(hktan.param("notlasttan"), Some("N"));
    assert_eq!(hktan.param("orderhash"), Some(expected_hash.as_str()));
    assert_eq!(
        hktan.lowlevel_param("TAN2Step5.orderhash"),
        Some(expected_lowlevel_hash.as_str())
    );
    assert_eq!(
        hktan.param("orderaccount.iban"),
        Some("DE02123456780000000000")
    );
    assert_eq!(hktan.param("tanmedia"), Some("sms-name"));

    handler
        .try_add_to_queue(hktan)
        .expect("prepared HKTAN verifies and queues");
    let status = handler.execute().await.expect("replay response");
    assert!(status.success);

    let requests = replay.requests().expect("requests");
    let body = &requests[0].body;
    let rendered = String::from_utf8_lossy(body);
    assert_signed_custom_msg_request_bytes(body, "0", "1", 5);
    assert!(rendered.contains("HKTAN:3:5+1+HKSAL+DE02123456780000000000"));
    assert!(rendered.contains("sms-name'"), "{rendered}");

    let hash_prefix = b"+@20@";
    let hash_start = find_bytes(body, hash_prefix).expect("HKTAN hash prefix");
    let payload_start = hash_start + hash_prefix.len();
    let payload_end = payload_start + 20;
    assert_eq!(
        &body[payload_start..payload_end],
        OrderHashMode::Sha1
            .hash_segment_bytes(ORDER_SEGMENT)
            .expect("expected hash")
    );
}

#[tokio::test]
async fn handler_prepares_process1_hktan_asks_callback_for_required_tan_media() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(TanMediaSelectingCallback {
            events: events.clone(),
            selection: Some("push-app".to_owned()),
        }),
    )
    .expect("runtime init");

    let passport = PinTanPassport::new(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        tan_media_names: vec!["mobile".to_owned(), "push-app".to_owned()],
        bpd_parameters: BTreeMap::from([
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.orderhashmode".to_owned(),
                "2".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.needtanmedia".to_owned(),
                "2".to_owned(),
            ),
        ]),
        ..PinTanPassportData::default()
    });
    let mut handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo
        .try_set_param("my.iban", "DE02123456780000000000")
        .expect("saldo account");

    let hktan = handler
        .new_tan2step_process1_job_with_tan_media_selection(&saldo, None)
        .await
        .expect("process-1 HKTAN prepares");

    assert_eq!(hktan.param("tanmedia"), Some("push-app"));
    assert_eq!(handler.passport().tan_media(), Some("push-app"));
    let events = events.lock().expect("callback event lock");
    let media_event = events
        .iter()
        .find(|event| event.reason == CallbackReason::NeedPtTanMedia)
        .expect("tan media callback event");
    assert_eq!(media_event.data_type, CallbackDataType::Text);
    assert_eq!(media_event.message, "*** Enter the name of your TAN media");
    assert_eq!(
        media_event.current_value.as_deref(),
        Some("mobile|push-app")
    );
    drop(events);
    done().expect("runtime reset");
}

#[test]
fn handler_prepares_process1_hktan_uses_noref_for_required_tan_media_without_value() {
    let passport = PinTanPassport::new(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.orderhashmode".to_owned(),
                "2".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.needtanmedia".to_owned(),
                "2".to_owned(),
            ),
        ]),
        ..PinTanPassportData::default()
    });
    let handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo
        .try_set_param("my.iban", "DE02123456780000000000")
        .expect("saldo account");

    let hktan = handler
        .new_tan2step_process1_job(&saldo, None)
        .expect("process-1 HKTAN prepares");

    assert_eq!(hktan.param("tanmedia"), Some("noref"));
    assert_eq!(handler.passport().tan_media(), None);
}

#[test]
fn handler_dispatches_initial_hktan_to_process1_from_bpd_process() {
    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        tan_media: Some("photo-app".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "1".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.orderhashmode".to_owned(),
                "2".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo
        .try_set_param("my.iban", "DE02123456780000000000")
        .expect("saldo account");

    let hktan = handler
        .new_tan2step_initial_job(&saldo, None)
        .expect("initial HKTAN prepares");

    assert_eq!(hktan.param("process"), Some("1"));
    assert_eq!(hktan.param("ordersegcode"), Some("HKSAL"));
    assert_eq!(hktan.param("tanmedia"), Some("photo-app"));
    assert!(hktan.param("orderhash").is_some());
    assert_eq!(hktan.param("notlasttan"), Some("N"));
}

#[tokio::test]
async fn handler_executes_process1_flow_automatically_and_merges_status() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(FixedTanCallback {
            events: events.clone(),
            tan: "987654".to_owned(),
        }),
    )
    .expect("runtime init");

    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        tan_media: Some("photo-app".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.segcode".to_owned(),
                "HKSAL".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.needtan".to_owned(),
                "J".to_owned(),
            ),
            (
                "Params.SaldoPar7.SegHead.code".to_owned(),
                "HISALS".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "1".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.orderhashmode".to_owned(),
                "2".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.needorderaccount".to_owned(),
                "2".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.name".to_owned(),
                "photoTAN".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.inputinfo".to_owned(),
                "Bitte bestaetigen".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&[
            "HIRMG:2:2+0010::OK",
            "HITAN:3:5+1++ORDER-REF-P1+Bitte geben Sie die TAN ein+@5@HHDUC",
        ])),
        Ok(custom_msg_response_for_request(
            "0",
            2,
            &[
                "HIRMG:2:2+0010::OK",
                "HISAL:3:7+DE02123456780000000000:MARKDEF1100+Girokonto+EUR+C:123,45:EUR:20260605",
            ],
        )),
    ]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &giro_account());

    let status = handler
        .execute_with_tan2step_process1(saldo)
        .await
        .expect("automatic process-1 flow executes");

    assert!(status.success);
    assert_eq!(
        status
            .job_results
            .iter()
            .map(|result| result.job_name.as_str())
            .collect::<Vec<_>>(),
        ["TAN2Step", "SaldoReq"]
    );
    assert_eq!(
        status.messages,
        vec!["0010:OK".to_owned(), "0010:OK".to_owned()]
    );
    let Some(HbciJobResultData::SaldoReq(result)) = status.job_results[1].result.as_ref() else {
        panic!("expected SaldoReq result data");
    };
    assert_eq!(result.entries[0].ready.value.value, "123.45");
    assert_eq!(handler.passport().sca_state().challenge, None);

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 2);
    let hktan_body = &requests[0].body;
    let hktan_text = String::from_utf8_lossy(hktan_body);
    assert_signed_custom_msg_request_bytes(hktan_body, "0", "1", 5);
    assert!(hktan_text.contains("HKTAN:3:5+1+HKSAL+DE02123456780000000000"));
    assert!(hktan_text.contains("photo-app'"), "{hktan_text}");
    assert!(!hktan_text.contains("HKSAL:3:7+"), "{hktan_text}");
    assert!(find_bytes(hktan_body, b"+@20@").is_some(), "{hktan_text}");

    let order_body = String::from_utf8(requests[1].body.clone()).expect("order body is text");
    assert!(order_body.contains("+300+0+2'"), "{order_body}");
    assert!(
        order_body
            .contains("HKSAL:3:7+DE02123456780000000000:MARKDEF1100:0001234567::280:12345678+N'")
    );
    assert!(!order_body.contains("HKTAN:"), "{order_body}");
    let sig_tail = fints_segment(&order_body, "HNSHA");
    let sig_tail_parts = sig_tail.split('+').collect::<Vec<_>>();
    assert_eq!(sig_tail_parts.get(3).copied(), Some("12345:987654"));

    let events = events.lock().expect("callback event lock");
    assert!(events.iter().any(|event| {
        event.reason == CallbackReason::NeedPtTan && event.current_value.as_deref() == Some("HHDUC")
    }));
    drop(events);
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_rejects_process1_auto_execution_when_queue_is_not_empty() {
    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        bpd_parameters: BTreeMap::from([(
            "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
            "1".to_owned(),
        )]),
        ..signed_pintan_data()
    });
    let mut handler = HbciHandler::new("300", passport);
    let mut queued = handler.new_job("SaldoReq").expect("job is in registry");
    queued.set_param_account("my", &giro_account());
    handler
        .try_add_to_queue(queued)
        .expect("business task queues");
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &giro_account());

    let err = handler
        .execute_with_tan2step_process1(saldo)
        .await
        .expect_err("non-empty queue is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(err.message(), "process-1 execution requires an empty queue");
    assert_eq!(handler.queued_jobs().len(), 1);
}

#[tokio::test]
async fn handler_dispatcher_executes_process1_queued_job_from_bpd_process() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(FixedTanCallback {
            events: events.clone(),
            tan: "987654".to_owned(),
        }),
    )
    .expect("runtime init");

    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        tan_media: Some("photo-app".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.segcode".to_owned(),
                "HKSAL".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.needtan".to_owned(),
                "J".to_owned(),
            ),
            (
                "Params.SaldoPar7.SegHead.code".to_owned(),
                "HISALS".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "1".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.orderhashmode".to_owned(),
                "2".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.name".to_owned(),
                "photoTAN".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.inputinfo".to_owned(),
                "Bitte bestaetigen".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&[
            "HIRMG:2:2+0010::OK",
            "HITAN:3:5+1++ORDER-REF-P1-DISPATCH+Bitte geben Sie die TAN ein+@5@HHDUC",
        ])),
        Ok(custom_msg_response_for_request(
            "0",
            2,
            &[
                "HIRMG:2:2+0010::OK",
                "HISAL:3:7+DE02123456780000000000:MARKDEF1100+Girokonto+EUR+C:123,45:EUR:20260605",
            ],
        )),
    ]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &giro_account());
    handler
        .try_add_to_queue(saldo)
        .expect("business task queues");

    let status = handler
        .execute_with_tan2step()
        .await
        .expect("dispatcher executes process-1 flow");

    assert!(status.success);
    assert_eq!(
        status
            .job_results
            .iter()
            .map(|result| result.job_name.as_str())
            .collect::<Vec<_>>(),
        ["TAN2Step", "SaldoReq"]
    );

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 2);
    let first = String::from_utf8_lossy(&requests[0].body);
    let second = String::from_utf8(requests[1].body.clone()).expect("second request is text");
    assert!(fints_segment(&first, "HKTAN").starts_with("HKTAN:3:5+1+HKSAL"));
    assert!(!first.contains("HKSAL:3:7+"), "{first}");
    assert!(second.contains("HKSAL:3:7+DE02123456780000000000"));
    assert!(!second.contains("HKTAN:"), "{second}");
    assert_eq!(
        fints_segment(&second, "HNSHA")
            .split('+')
            .collect::<Vec<_>>()
            .get(3)
            .copied(),
        Some("12345:987654")
    );
    assert!(
        events
            .lock()
            .expect("callback event lock")
            .iter()
            .any(|event| {
                event.reason == CallbackReason::NeedPtTan
                    && event.current_value.as_deref() == Some("HHDUC")
            })
    );
    done().expect("runtime reset");
}

#[test]
fn handler_dispatches_initial_hktan_to_process2_step1_from_bpd_process() {
    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        tan_media: Some("push-app".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "2".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo
        .try_set_param("my.iban", "DE02123456780000000000")
        .expect("saldo account");

    let hktan = handler
        .new_tan2step_initial_job(&saldo, None)
        .expect("initial HKTAN prepares");

    assert_eq!(hktan.param("process"), Some("4"));
    assert_eq!(hktan.param("ordersegcode"), Some("HKSAL"));
    assert_eq!(hktan.param("tanmedia"), Some("push-app"));
    assert_eq!(hktan.param("orderhash"), None);
    assert_eq!(hktan.param("notlasttan"), None);
}

#[test]
fn handler_dispatches_initial_hktan_unknown_process_to_process2_step1_like_original() {
    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        tan_media: Some("push-app".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "X".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo
        .try_set_param("my.iban", "DE02123456780000000000")
        .expect("saldo account");

    let hktan = handler
        .new_tan2step_initial_job(&saldo, None)
        .expect("initial HKTAN prepares");

    assert_eq!(hktan.param("process"), Some("4"));
    assert_eq!(hktan.param("ordersegcode"), Some("HKSAL"));
    assert_eq!(hktan.param("orderhash"), None);
}

#[tokio::test]
async fn handler_prepares_process2_step1_hktan_next_to_original_task() {
    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        tan_media: Some("push-app".to_owned()),
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_ok_response())]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo
        .try_set_param("my.iban", "DE02123456780000000000")
        .expect("saldo account");

    let hktan = handler
        .new_tan2step_process2_step1_job(&saldo)
        .expect("process-2 step-1 HKTAN prepares");

    assert_eq!(hktan.name(), "TAN2Step");
    assert_eq!(hktan.param("process"), Some("4"));
    assert_eq!(hktan.param("ordersegcode"), Some("HKSAL"));
    assert_eq!(hktan.param("tanmedia"), Some("push-app"));
    assert_eq!(hktan.param("orderhash"), None);
    assert_eq!(hktan.param("orderref"), None);
    assert_eq!(hktan.param("notlasttan"), None);

    handler.add_to_queue(saldo);
    handler
        .try_add_to_queue(hktan)
        .expect("prepared HKTAN verifies and queues");
    let status = handler.execute().await.expect("replay response");
    assert!(status.success);

    let requests = replay.requests().expect("requests");
    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert_signed_custom_msg_request(&body, "0", "1", 6);
    assert!(body.contains("HKSAL:3:7+DE02123456780000000000+N'"));

    let hktan_segment = fints_segment(&body, "HKTAN");
    let fields = hktan_segment.split('+').collect::<Vec<_>>();
    assert_eq!(fields.get(1).copied(), Some("4"));
    assert_eq!(fields.get(2).copied(), Some("HKSAL"));
    assert_eq!(fields.last().copied(), Some("push-app"));
    assert!(!hktan_segment.contains("@20@"), "{hktan_segment}");
}

#[tokio::test]
async fn handler_prepares_process2_step1_hktan_asks_callback_for_required_tan_media() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(TanMediaSelectingCallback {
            events: events.clone(),
            selection: Some("mobiletan".to_owned()),
        }),
    )
    .expect("runtime init");

    let passport = PinTanPassport::new(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        tan_media_names: vec!["mobiletan".to_owned()],
        bpd_parameters: BTreeMap::from([
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.needtanmedia".to_owned(),
                "2".to_owned(),
            ),
        ]),
        ..PinTanPassportData::default()
    });
    let mut handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo
        .try_set_param("my.iban", "DE02123456780000000000")
        .expect("saldo account");

    let hktan = handler
        .new_tan2step_process2_step1_job_with_tan_media_selection(&saldo)
        .await
        .expect("process-2 step-1 HKTAN prepares");

    assert_eq!(hktan.param("process"), Some("4"));
    assert_eq!(hktan.param("ordersegcode"), Some("HKSAL"));
    assert_eq!(hktan.param("tanmedia"), Some("mobiletan"));
    assert_eq!(handler.passport().tan_media(), Some("mobiletan"));
    let events = events.lock().expect("callback event lock");
    assert!(
        events
            .iter()
            .any(|event| event.reason == CallbackReason::NeedPtTanMedia)
    );
    drop(events);
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_auto_queues_process2_hktan_for_tan_required_task() {
    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        tan_media: Some("push-app".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.segcode".to_owned(),
                "HKSAL".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.needtan".to_owned(),
                "J".to_owned(),
            ),
            (
                "Params.SaldoPar7.SegHead.code".to_owned(),
                "HISALS".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "2".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_ok_response())]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &giro_account());

    handler
        .try_add_to_queue_with_initial_tan_job(saldo)
        .expect("task and HKTAN queue");

    assert_eq!(handler.queued_jobs().len(), 2);
    assert_eq!(handler.queued_jobs()[0].name(), "SaldoReq");
    assert_eq!(handler.queued_jobs()[1].name(), "TAN2Step");
    assert_eq!(handler.queued_jobs()[1].param("process"), Some("4"));
    assert_eq!(
        handler.queued_jobs()[1].param("ordersegcode"),
        Some("HKSAL")
    );
    assert_eq!(handler.queued_jobs()[1].param("tanmedia"), Some("push-app"));

    let status = handler.execute().await.expect("replay response");
    assert!(status.success);

    let requests = replay.requests().expect("requests");
    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");
    assert_signed_custom_msg_request(&body, "0", "1", 6);
    assert!(
        body.contains("HKSAL:3:7+DE02123456780000000000:MARKDEF1100:0001234567::280:12345678+N'")
    );
    let hktan = fints_segment(&body, "HKTAN");
    let fields = hktan.split('+').collect::<Vec<_>>();
    assert_eq!(fields.get(1).copied(), Some("4"));
    assert_eq!(fields.get(2).copied(), Some("HKSAL"));
    assert_eq!(fields.last().copied(), Some("push-app"));
}

#[test]
fn handler_auto_queue_keeps_non_tan_required_task_without_hktan() {
    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.segcode".to_owned(),
                "HKSAL".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.needtan".to_owned(),
                "N".to_owned(),
            ),
            (
                "Params.SaldoPar7.SegHead.code".to_owned(),
                "HISALS".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "2".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let mut handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &giro_account());

    handler
        .try_add_to_queue_with_initial_tan_job(saldo)
        .expect("only task queues");

    assert_eq!(handler.queued_jobs().len(), 1);
    assert_eq!(handler.queued_jobs()[0].name(), "SaldoReq");
}

#[test]
fn handler_auto_queue_rejects_process1_tan_required_task_until_multimessage_support() {
    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.segcode".to_owned(),
                "HKSAL".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.needtan".to_owned(),
                "J".to_owned(),
            ),
            (
                "Params.SaldoPar7.SegHead.code".to_owned(),
                "HISALS".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "1".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.orderhashmode".to_owned(),
                "2".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let mut handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &giro_account());

    let err = handler
        .try_add_to_queue_with_initial_tan_job(saldo)
        .expect_err("process 1 auto queueing is not available yet");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::Unsupported);
    assert_eq!(
        err.message(),
        "process-1 automatic HKTAN queueing requires multi-message execution"
    );
    assert!(handler.queued_jobs().is_empty());
}

#[tokio::test]
async fn handler_prepares_process2_hktan_from_stored_hitan_orderref() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HITAN:3:5+1++ORDER-REF-2+Bitte geben Sie die TAN ein+@5@HHDUC",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);
    let mut first = handler.new_job("TAN2Step").expect("job is in registry");
    first
        .try_set_param("process", "4")
        .expect("process is accepted");
    handler.add_to_queue(first);
    handler.execute().await.expect("hitan response parses");

    let mut second = handler
        .new_tan2step_process2_job()
        .expect("process-2 HKTAN prepares");
    let lowlevel = second.verify_constraints().expect("HKTAN verifies");

    assert_eq!(second.name(), "TAN2Step");
    assert_eq!(second.param("process"), Some("2"));
    assert_eq!(second.param("orderref"), Some("ORDER-REF-2"));
    assert_eq!(second.param("notlasttan"), Some("N"));
    assert_eq!(
        lowlevel.get("TAN2Step5.process").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        lowlevel.get("TAN2Step5.orderref").map(String::as_str),
        Some("ORDER-REF-2")
    );
    assert_eq!(
        lowlevel.get("TAN2Step5.notlasttan").map(String::as_str),
        Some("N")
    );
}

#[test]
fn handler_rejects_process2_hktan_without_stored_orderref() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let handler = HbciHandler::new("300", passport);

    let err = handler
        .new_tan2step_process2_job()
        .expect_err("missing order reference is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(
        err.message(),
        "PinTAN SCA state does not contain an order reference for process-2 HKTAN"
    );
}

#[tokio::test]
async fn handler_executes_process2_tan_submission_from_stored_hitan_state() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(FixedTanCallback {
            events: events.clone(),
            tan: "987654".to_owned(),
        }),
    )
    .expect("runtime init");

    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.segcode".to_owned(),
                "HKSAL".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.needtan".to_owned(),
                "J".to_owned(),
            ),
            (
                "Params.SaldoPar7.SegHead.code".to_owned(),
                "HISALS".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "2".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.name".to_owned(),
                "pushTAN".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.inputinfo".to_owned(),
                "Bitte bestaetigen".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&[
            "HIRMG:2:2+0010::OK",
            "HITAN:3:5+1++ORDER-REF-99+Bitte geben Sie die TAN ein+@5@HHDUC",
        ])),
        Ok(custom_msg_response_for_request(
            "0",
            2,
            &["HIRMG:2:2+0010::OK"],
        )),
    ]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &giro_account());

    handler
        .try_add_to_queue_with_initial_tan_job(saldo)
        .expect("task and first HKTAN queue");
    let first_status = handler.execute().await.expect("first replay response");
    assert!(first_status.success);
    assert_eq!(
        handler.passport().sca_state().order_ref.as_deref(),
        Some("ORDER-REF-99")
    );

    let second_status = handler
        .execute_tan2step_process2_submission()
        .await
        .expect("process-2 TAN submission executes");

    assert!(second_status.success);
    assert_eq!(second_status.job_results.len(), 1);
    assert_eq!(second_status.job_results[0].job_name, "TAN2Step");
    assert_eq!(handler.passport().sca_state().order_ref, None);
    assert_eq!(handler.passport().sca_state().challenge, None);
    assert_eq!(handler.passport().sca_state().hhd_uc, None);

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 2);
    let first_body = String::from_utf8(requests[0].body.clone()).expect("first body is text");
    assert!(fints_segment(&first_body, "HKTAN").starts_with("HKTAN:4:5+4+HKSAL"));

    let second_body = String::from_utf8(requests[1].body.clone()).expect("second body is text");
    assert!(second_body.contains("+300+0+2'"), "{second_body}");
    assert!(fints_segment(&second_body, "HKTAN").starts_with("HKTAN:3:5+2"));
    assert!(
        fints_segment(&second_body, "HKTAN").contains("ORDER-REF-99"),
        "{second_body}"
    );
    let sig_tail = fints_segment(&second_body, "HNSHA");
    let sig_tail_parts = sig_tail.split('+').collect::<Vec<_>>();
    assert_eq!(sig_tail_parts.get(3).copied(), Some("12345:987654"));

    let events = events.lock().expect("callback event lock");
    let tan_event = events
        .iter()
        .find(|event| {
            event.reason == CallbackReason::NeedPtTan
                && event.message == "pushTAN\nBitte bestaetigen\n\nBitte geben Sie die TAN ein"
        })
        .expect("process-2 TAN callback event");
    assert_eq!(tan_event.data_type, CallbackDataType::Text);
    assert_eq!(tan_event.current_value.as_deref(), Some("HHDUC"));
    drop(events);
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_executes_process2_flow_automatically_and_merges_status() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(FixedTanCallback {
            events: events.clone(),
            tan: "987654".to_owned(),
        }),
    )
    .expect("runtime init");

    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.segcode".to_owned(),
                "HKSAL".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.needtan".to_owned(),
                "J".to_owned(),
            ),
            (
                "Params.SaldoPar7.SegHead.code".to_owned(),
                "HISALS".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "2".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.name".to_owned(),
                "pushTAN".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.inputinfo".to_owned(),
                "Bitte bestaetigen".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&[
            "HIRMG:2:2+0010::OK",
            "HITAN:3:5+1++ORDER-REF-100+Bitte geben Sie die TAN ein+@5@HHDUC",
        ])),
        Ok(custom_msg_response_for_request(
            "0",
            2,
            &["HIRMG:2:2+0010::OK"],
        )),
    ]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &giro_account());

    handler
        .try_add_to_queue_with_initial_tan_job(saldo)
        .expect("task and first HKTAN queue");

    let status = handler
        .execute_with_tan2step_process2()
        .await
        .expect("automatic process-2 flow executes");

    assert!(status.success);
    assert_eq!(
        status
            .job_results
            .iter()
            .map(|result| result.job_name.as_str())
            .collect::<Vec<_>>(),
        ["SaldoReq", "TAN2Step", "TAN2Step"]
    );
    assert_eq!(
        status.messages,
        vec!["0010:OK".to_owned(), "0010:OK".to_owned()]
    );
    assert_eq!(status.global_return_values.len(), 2);
    assert_eq!(handler.passport().sca_state().order_ref, None);

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 2);
    let first_body = String::from_utf8(requests[0].body.clone()).expect("first body is text");
    let second_body = String::from_utf8(requests[1].body.clone()).expect("second body is text");
    assert!(fints_segment(&first_body, "HKTAN").starts_with("HKTAN:4:5+4+HKSAL"));
    assert!(second_body.contains("+300+0+2'"), "{second_body}");
    assert!(fints_segment(&second_body, "HKTAN").starts_with("HKTAN:3:5+2"));
    assert!(
        fints_segment(&second_body, "HKTAN").contains("ORDER-REF-100"),
        "{second_body}"
    );

    let events = events.lock().expect("callback event lock");
    assert!(events.iter().any(|event| {
        event.reason == CallbackReason::NeedPtTan && event.current_value.as_deref() == Some("HHDUC")
    }));
    drop(events);
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_dispatcher_inserts_process2_hktan_for_queued_job() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(FixedTanCallback {
            events: events.clone(),
            tan: "987654".to_owned(),
        }),
    )
    .expect("runtime init");

    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        tan_media: Some("push-app".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.segcode".to_owned(),
                "HKSAL".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.needtan".to_owned(),
                "J".to_owned(),
            ),
            (
                "Params.SaldoPar7.SegHead.code".to_owned(),
                "HISALS".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "2".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.name".to_owned(),
                "pushTAN".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.inputinfo".to_owned(),
                "Bitte bestaetigen".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([
        Ok(custom_msg_response(&[
            "HIRMG:2:2+0010::OK",
            "HITAN:3:5+1++ORDER-REF-P2-DISPATCH+Bitte geben Sie die TAN ein+@5@HHDUC",
        ])),
        Ok(custom_msg_response_for_request(
            "0",
            2,
            &["HIRMG:2:2+0010::OK"],
        )),
    ]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &giro_account());
    handler
        .try_add_to_queue(saldo)
        .expect("business task queues");

    let status = handler
        .execute_with_tan2step()
        .await
        .expect("dispatcher executes process-2 flow");

    assert!(status.success);
    assert_eq!(
        status
            .job_results
            .iter()
            .map(|result| result.job_name.as_str())
            .collect::<Vec<_>>(),
        ["SaldoReq", "TAN2Step", "TAN2Step"]
    );

    let requests = replay.requests().expect("requests");
    assert_eq!(requests.len(), 2);
    let first = String::from_utf8(requests[0].body.clone()).expect("first request is text");
    let second = String::from_utf8(requests[1].body.clone()).expect("second request is text");
    assert!(first.contains("HKSAL:3:7+DE02123456780000000000"));
    assert!(fints_segment(&first, "HKTAN").starts_with("HKTAN:4:5+4+HKSAL"));
    assert!(fints_segment(&second, "HKTAN").starts_with("HKTAN:3:5+2"));
    assert!(
        fints_segment(&second, "HKTAN").contains("ORDER-REF-P2-DISPATCH"),
        "{second}"
    );
    assert_eq!(
        fints_segment(&second, "HNSHA")
            .split('+')
            .collect::<Vec<_>>()
            .get(3)
            .copied(),
        Some("12345:987654")
    );
    assert!(
        events
            .lock()
            .expect("callback event lock")
            .iter()
            .any(|event| {
                event.reason == CallbackReason::NeedPtTan
                    && event.current_value.as_deref() == Some("HHDUC")
            })
    );
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_process2_auto_execution_stops_when_no_orderref_was_returned() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(FixedTanCallback {
            events: events.clone(),
            tan: "987654".to_owned(),
        }),
    )
    .expect("runtime init");

    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.segcode".to_owned(),
                "HKSAL".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.needtan".to_owned(),
                "J".to_owned(),
            ),
            (
                "Params.SaldoPar7.SegHead.code".to_owned(),
                "HISALS".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.process".to_owned(),
                "2".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&["HIRMG:2:2+0010::OK"]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &giro_account());

    handler
        .try_add_to_queue_with_initial_tan_job(saldo)
        .expect("task and first HKTAN queue");

    let status = handler
        .execute_with_tan2step_process2()
        .await
        .expect("first message executes");

    assert!(status.success);
    assert_eq!(status.job_results.len(), 2);
    assert_eq!(replay.requests().expect("requests").len(), 1);
    assert!(
        events
            .lock()
            .expect("callback event lock")
            .iter()
            .all(|event| event.reason != CallbackReason::NeedPtTan)
    );
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_rejects_process2_tan_submission_when_queue_is_not_empty() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let mut handler = HbciHandler::new("300", passport);
    let mut saldo = handler.new_job("SaldoReq").expect("job is in registry");
    saldo.set_param_account("my", &giro_account());
    handler
        .try_add_to_queue(saldo)
        .expect("business task queues");

    let err = handler
        .execute_tan2step_process2_submission()
        .await
        .expect_err("non-empty queue is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(
        err.message(),
        "process-2 TAN submission requires an empty queue"
    );
    assert_eq!(handler.queued_jobs().len(), 1);
}

#[tokio::test]
async fn handler_execute_imports_hitan_sca_state_from_tan2step_response() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HITAN:3:5+1++ORDER-REF-1+Bitte geben Sie die TAN ein+@5@HHDUC",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);
    let mut hktan = handler.new_job("TAN2Step").expect("job is in registry");
    hktan
        .try_set_param("process", "1")
        .expect("process is accepted");
    handler.add_to_queue(hktan);

    let status = handler.execute().await.expect("hitan response parses");

    assert!(status.success);
    let sca = handler.passport().sca_state();
    assert!(!sca.sca_exempted);
    assert_eq!(
        sca.challenge.as_deref(),
        Some("Bitte geben Sie die TAN ein")
    );
    assert_eq!(sca.hhd_uc.as_deref(), Some("HHDUC"));
    assert_eq!(sca.order_ref.as_deref(), Some("ORDER-REF-1"));
}

#[tokio::test]
async fn handler_execute_ignores_nochallenge_but_keeps_hitan_orderref() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HITAN:3:5+1++ORDER-REF-2+nochallenge+@5@HHDUC",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);
    let mut hktan = handler.new_job("TAN2Step").expect("job is in registry");
    hktan
        .try_set_param("process", "1")
        .expect("process is accepted");
    handler.add_to_queue(hktan);

    handler.execute().await.expect("hitan response parses");

    let sca = handler.passport().sca_state();
    assert_eq!(sca.challenge, None);
    assert_eq!(sca.hhd_uc.as_deref(), Some("HHDUC"));
    assert_eq!(sca.order_ref.as_deref(), Some("ORDER-REF-2"));
}

#[tokio::test]
async fn handler_execute_marks_sca_exempted_for_3076_response() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+3076::Keine starke Kundenauthentifizierung erforderlich",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);
    let mut hktan = handler.new_job("TAN2Step").expect("job is in registry");
    hktan
        .try_set_param("process", "1")
        .expect("process is accepted");
    handler.add_to_queue(hktan);

    handler.execute().await.expect("3076 response parses");

    let sca = handler.passport().sca_state();
    assert!(sca.sca_exempted);
    assert_eq!(sca.challenge, None);
    assert_eq!(sca.hhd_uc, None);
    assert_eq!(sca.order_ref, None);
}

#[tokio::test]
async fn handler_requests_tan_for_stored_sca_challenge() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(ScriptedCallback {
            events: events.clone(),
            responses: Arc::new(Mutex::new(VecDeque::from([CallbackResponse::value(
                "123456",
            )]))),
        }),
    )
    .expect("runtime init");

    let passport = passport_with_cached_pin(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.name".to_owned(),
                "pushTAN".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.inputinfo".to_owned(),
                "Bitte bestaetigen".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HITAN:3:5+1++ORDER-REF-1+Bitte geben Sie die TAN ein+@5@HHDUC",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);
    let mut hktan = handler.new_job("TAN2Step").expect("job is in registry");
    hktan
        .try_set_param("process", "1")
        .expect("process is accepted");
    handler.add_to_queue(hktan);
    handler.execute().await.expect("hitan response parses");

    let tan = handler
        .request_tan_for_sca()
        .await
        .expect("tan callback succeeds");

    assert_eq!(tan.as_deref(), Some("123456"));
    let events = events.lock().expect("callback event lock");
    let tan_event = events
        .iter()
        .find(|event| event.reason == CallbackReason::NeedPtTan)
        .expect("tan callback event");
    assert_eq!(tan_event.data_type, CallbackDataType::Text);
    assert_eq!(
        tan_event.message,
        "pushTAN\nBitte bestaetigen\n\nBitte geben Sie die TAN ein"
    );
    assert_eq!(tan_event.current_value.as_deref(), Some("HHDUC"));
    drop(events);
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_does_not_request_tan_for_3076_sca_exemption() {
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

    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+3076::Keine starke Kundenauthentifizierung erforderlich",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);
    let mut hktan = handler.new_job("TAN2Step").expect("job is in registry");
    hktan
        .try_set_param("process", "1")
        .expect("process is accepted");
    handler.add_to_queue(hktan);
    handler.execute().await.expect("3076 response parses");

    let tan = handler
        .request_tan_for_sca()
        .await
        .expect("3076 does not need TAN");

    assert_eq!(tan, None);
    assert!(
        events
            .lock()
            .expect("callback event lock")
            .iter()
            .all(|event| event.reason != CallbackReason::NeedPtTan)
    );
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_errors_when_tan_callback_returns_empty_value() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(ScriptedCallback {
            events,
            responses: Arc::new(Mutex::new(VecDeque::from([CallbackResponse::empty()]))),
        }),
    )
    .expect("runtime init");

    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HITAN:3:5+1++ORDER-REF-1+Bitte geben Sie die TAN ein+@5@HHDUC",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);
    let mut hktan = handler.new_job("TAN2Step").expect("job is in registry");
    hktan
        .try_set_param("process", "1")
        .expect("process is accepted");
    handler.add_to_queue(hktan);
    handler.execute().await.expect("hitan response parses");

    let err = handler
        .request_tan_for_sca()
        .await
        .expect_err("empty TAN is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::Callback);
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_requests_pin_once_and_caches_response() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(ScriptedCallback {
            events: events.clone(),
            responses: Arc::new(Mutex::new(VecDeque::from([CallbackResponse::value(
                "12345",
            )]))),
        }),
    )
    .expect("runtime init");

    let passport = PinTanPassport::new(PinTanPassportData::default());
    let mut handler = HbciHandler::new("300", passport);

    let first = handler.request_pin().await.expect("PIN callback succeeds");
    let second = handler.request_pin().await.expect("cached PIN is reused");

    assert_eq!(first, "12345");
    assert_eq!(second, "12345");
    assert_eq!(handler.passport().pin(), Some("12345"));

    let events = events.lock().expect("callback event lock");
    let pin_events = events
        .iter()
        .filter(|event| event.reason == CallbackReason::NeedPtPin)
        .collect::<Vec<_>>();
    assert_eq!(pin_events.len(), 1);
    assert_eq!(pin_events[0].data_type, CallbackDataType::Secret);
    assert_eq!(
        pin_events[0].message,
        "Please enter your PIN for PIN/TAN now"
    );
    assert_eq!(pin_events[0].current_value, None);
    drop(events);
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_reuses_cached_pin_without_callback() {
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

    let mut passport = PinTanPassport::new(PinTanPassportData::default());
    passport.set_pin("cached-pin");
    let mut handler = HbciHandler::new("300", passport);

    let pin = handler.request_pin().await.expect("cached PIN is reused");

    assert_eq!(pin, "cached-pin");
    assert!(
        events
            .lock()
            .expect("callback event lock")
            .iter()
            .all(|event| event.reason != CallbackReason::NeedPtPin)
    );
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_errors_when_pin_callback_returns_empty_value() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(ScriptedCallback {
            events,
            responses: Arc::new(Mutex::new(VecDeque::from([CallbackResponse::empty()]))),
        }),
    )
    .expect("runtime init");

    let passport = PinTanPassport::new(PinTanPassportData::default());
    let mut handler = HbciHandler::new("300", passport);

    let err = handler
        .request_pin()
        .await
        .expect_err("empty PIN is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::Callback);
    assert_eq!(handler.passport().pin(), None);
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_signs_pintan_usersig_for_sca_challenge() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(ScriptedCallback {
            events: events.clone(),
            responses: Arc::new(Mutex::new(VecDeque::from([
                CallbackResponse::value("12345"),
                CallbackResponse::value("987654"),
            ]))),
        }),
    )
    .expect("runtime init");

    let passport = PinTanPassport::new(PinTanPassportData {
        tan_method: Some("921".to_owned()),
        bpd_parameters: BTreeMap::from([
            (
                "Params.TAN2StepPar5.ParTAN2Step.secfunc".to_owned(),
                "921".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.name".to_owned(),
                "pushTAN".to_owned(),
            ),
            (
                "Params.TAN2StepPar5.ParTAN2Step.inputinfo".to_owned(),
                "Bitte bestaetigen".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HITAN:3:5+1++ORDER-REF-1+Bitte geben Sie die TAN ein+@5@HHDUC",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);
    let mut hktan = handler.new_job("TAN2Step").expect("job is in registry");
    hktan
        .try_set_param("process", "1")
        .expect("process is accepted");
    handler.add_to_queue(hktan);
    handler.execute().await.expect("hitan response parses");

    let signature = handler
        .sign_pintan_user_sig_for_sca()
        .await
        .expect("usersig is signed");
    let decoded = UserSig::decode(Some(&signature)).expect("usersig decodes");

    assert_eq!(decoded.pin(), "12345");
    assert_eq!(decoded.tan(), "987654");
    assert_eq!(handler.passport().pin(), Some("12345"));

    let events = events.lock().expect("callback event lock");
    let reasons = events.iter().map(|event| event.reason).collect::<Vec<_>>();
    assert_eq!(
        reasons,
        [CallbackReason::NeedPtPin, CallbackReason::NeedPtTan]
    );
    drop(events);
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_signs_pintan_usersig_without_tan_for_sca_exemption() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(ScriptedCallback {
            events: events.clone(),
            responses: Arc::new(Mutex::new(VecDeque::from([CallbackResponse::value(
                "12345",
            )]))),
        }),
    )
    .expect("runtime init");

    let passport = PinTanPassport::new(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+3076::Keine starke Kundenauthentifizierung erforderlich",
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);
    let mut hktan = handler.new_job("TAN2Step").expect("job is in registry");
    hktan
        .try_set_param("process", "1")
        .expect("process is accepted");
    handler.add_to_queue(hktan);
    handler.execute().await.expect("3076 response parses");

    let signature = handler
        .sign_pintan_user_sig_for_sca()
        .await
        .expect("usersig is signed without TAN");
    let decoded = UserSig::decode(Some(&signature)).expect("usersig decodes");

    assert_eq!(decoded.pin(), "12345");
    assert_eq!(decoded.tan(), "");
    assert!(
        events
            .lock()
            .expect("callback event lock")
            .iter()
            .all(|event| event.reason != CallbackReason::NeedPtTan)
    );
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_renders_kums_all_request_like_original() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_ok_response())]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut kums = handler.new_job("KUmsAll").expect("job is in registry");
    kums.set_param_account("my", &giro_account());
    kums.try_set_param_date("startdate", "2026-06-01")
        .expect("start date is accepted");
    kums.try_set_param_date("enddate", "2026-06-06")
        .expect("end date is accepted");
    kums.try_set_param_int("maxentries", 25)
        .expect("maxentries is accepted");

    handler.add_to_queue(kums);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "KUmsAll");
    assert!(status.job_results[0].result.is_none());

    let requests = replay.requests().expect("requests");
    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");

    assert!(
        body.contains("HKKAZ:3:7+DE02123456780000000000:MARKDEF1100:0001234567::280:12345678+N+20260601+20260606+25'")
    );
    assert_signed_custom_msg_request(&body, "0", "1", 5);
}

#[tokio::test]
async fn handler_collects_kums_all_raw_result_data() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let booked = mt940_booked_payload();
    let notbooked = mt942_unbooked_payload();
    let segment = kums_response_segment("HIKAZ", &booked, &notbooked);
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        segment.as_str(),
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);
    let mut kums = handler.new_job("KUmsAll").expect("job is in registry");
    kums.set_param_account("my", &giro_account());

    handler.add_to_queue(kums);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    let result = &status.job_results[0];
    assert_eq!(result.job_name, "KUmsAll");
    assert_eq!(
        result.result_data.get("content.booked").map(String::as_str),
        Some(booked.as_str())
    );
    assert_eq!(
        result
            .result_data
            .get("content.notbooked")
            .map(String::as_str),
        Some(notbooked.as_str())
    );

    let Some(HbciJobResultData::KUms(mut kums_result)) = result.result.clone() else {
        panic!("expected KUms result data");
    };
    {
        let booked_lines = kums_result.get_flat_data();
        assert_eq!(booked_lines.len(), 1);
        assert_eq!(booked_lines[0].text.as_deref(), Some("GUTSCHRIFT MÄLLER"));
        assert_eq!(booked_lines[0].usage, vec!["Booked usage"]);
        assert_eq!(
            booked_lines[0]
                .value
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("2.00 EUR")
        );
    }
    {
        let unbooked_lines = kums_result.get_flat_data_unbooked();
        assert_eq!(unbooked_lines.len(), 1);
        assert_eq!(unbooked_lines[0].text.as_deref(), Some("VORMERKUNG"));
        assert_eq!(unbooked_lines[0].usage, vec!["Unbooked usage"]);
        assert_eq!(
            unbooked_lines[0]
                .value
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("3.00 EUR")
        );
    }
}

#[tokio::test]
async fn handler_renders_kums_all_camt_request_like_original() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_ok_response())]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut kums = handler.new_job("KUmsAllCamt").expect("job is in registry");
    kums.set_param_account("my", &giro_account());
    kums.try_set_param_date("startdate", "2026-06-01")
        .expect("start date is accepted");
    kums.try_set_param_date("enddate", "2026-06-06")
        .expect("end date is accepted");
    kums.try_set_param_int("maxentries", 25)
        .expect("maxentries is accepted");
    kums.set_param("offset", "CURSOR");

    handler.add_to_queue(kums);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "KUmsAllCamt");
    assert!(status.job_results[0].result.is_none());

    let requests = replay.requests().expect("requests");
    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");

    assert!(
        body.contains("HKCAZ:3:1+DE02123456780000000000:MARKDEF1100:0001234567::280:12345678+")
    );
    assert!(body.contains("urn?:iso?:std?:iso?:20022?:tech?:xsd?:camt.052.001.01"));
    assert!(body.contains("+N+20260601+20260606+25+CURSOR'"));
    assert_signed_custom_msg_request(&body, "0", "1", 5);
}

#[tokio::test]
async fn handler_collects_kums_all_camt_raw_result_data() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let booked_first = camt_payload("booked-1");
    let booked_second = camt_payload("booked-2");
    let notbooked = camt_payload("notbooked");
    let segment = kums_camt_response_segment(&booked_first, &booked_second, &notbooked);
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        segment.as_str(),
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);
    let mut kums = handler.new_job("KUmsAllCamt").expect("job is in registry");
    kums.set_param_account("my", &giro_account());

    handler.add_to_queue(kums);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    let result = &status.job_results[0];
    assert_eq!(result.job_name, "KUmsAllCamt");
    assert_eq!(
        result.result_data.get("content.format").map(String::as_str),
        Some(CAMT_052_001_01_URN)
    );
    assert_eq!(
        result
            .result_data
            .get("content.booked.message")
            .map(String::as_str),
        Some(booked_first.as_str())
    );
    assert_eq!(
        result
            .result_data
            .get("content.booked.message_2")
            .map(String::as_str),
        Some(booked_second.as_str())
    );
    assert_eq!(
        result
            .result_data
            .get("content.notbooked")
            .map(String::as_str),
        Some(notbooked.as_str())
    );

    let Some(HbciJobResultData::KUms(kums_result)) = result.result.clone() else {
        panic!("expected KUms result data");
    };
    assert_eq!(kums_result.camt_booked, vec![booked_first, booked_second]);
    assert_eq!(kums_result.camt_not_booked, vec![notbooked]);
}

#[tokio::test]
async fn handler_rejects_kums_all_without_account_fallback() {
    let passport = PinTanPassport::new(PinTanPassportData {
        host: Some("https://fints.example.test/fints".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_ok_response())]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let job = handler.new_job("KUmsAll").expect("job is in registry");

    handler.add_to_queue(job);
    let err = handler
        .execute()
        .await
        .expect_err("missing KUmsAll account is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(
        err.message(),
        "KUmsAll requires my.iban, my.number, or a passport account for the current KUmsZeit7 tracer renderer"
    );
    assert_eq!(replay.requests().expect("requests").len(), 0);
}

#[tokio::test]
async fn handler_rejects_kums_all_camt_without_account_fallback() {
    let passport = PinTanPassport::new(PinTanPassportData {
        host: Some("https://fints.example.test/fints".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_ok_response())]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let job = handler.new_job("KUmsAllCamt").expect("job is in registry");

    handler.add_to_queue(job);
    let err = handler
        .execute()
        .await
        .expect_err("missing KUmsAllCamt account is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(
        err.message(),
        "KUmsAllCamt requires my.iban, my.number, or a passport account for the current KUmsZeitCamt1 tracer renderer"
    );
    assert_eq!(replay.requests().expect("requests").len(), 0);
}

#[tokio::test]
async fn handler_renders_kums_new_request_like_original() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_ok_response())]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let mut kums = handler.new_job("KUmsNew").expect("job is in registry");
    kums.set_param_account("my", &giro_account());
    kums.try_set_param_int("maxentries", 25)
        .expect("maxentries is accepted");

    handler.add_to_queue(kums);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    assert_eq!(status.job_results[0].job_name, "KUmsNew");
    assert!(status.job_results[0].result.is_none());

    let requests = replay.requests().expect("requests");
    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");

    assert!(
        body.contains(
            "HKKAN:3:7+DE02123456780000000000:MARKDEF1100:0001234567::280:12345678+N+25'"
        )
    );
    assert_signed_custom_msg_request(&body, "0", "1", 5);
}

#[tokio::test]
async fn handler_collects_kums_new_raw_result_data() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let booked = mt940_booked_payload();
    let notbooked = mt942_unbooked_payload();
    let segment = kums_response_segment("HIKAN", &booked, &notbooked);
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        segment.as_str(),
    ]))]);
    let mut handler = HbciHandler::with_comm("300", passport, replay);
    let mut kums = handler.new_job("KUmsNew").expect("job is in registry");
    kums.set_param_account("my", &giro_account());

    handler.add_to_queue(kums);
    let status = handler.execute().await.expect("replay response");

    assert!(status.success);
    let result = &status.job_results[0];
    assert_eq!(result.job_name, "KUmsNew");
    assert_eq!(
        result.result_data.get("content.booked").map(String::as_str),
        Some(booked.as_str())
    );
    assert_eq!(
        result
            .result_data
            .get("content.notbooked")
            .map(String::as_str),
        Some(notbooked.as_str())
    );

    let Some(HbciJobResultData::KUms(mut kums_result)) = result.result.clone() else {
        panic!("expected KUms result data");
    };
    assert_eq!(kums_result.get_flat_data().len(), 1);
    assert_eq!(kums_result.get_flat_data_unbooked().len(), 1);
}

#[tokio::test]
async fn handler_rejects_kums_new_without_account_fallback() {
    let passport = PinTanPassport::new(PinTanPassportData {
        host: Some("https://fints.example.test/fints".to_owned()),
        ..PinTanPassportData::default()
    });
    let replay = ReplayCommClient::new([Ok(custom_msg_ok_response())]);
    let mut handler = HbciHandler::with_comm("300", passport, replay.clone());
    let job = handler.new_job("KUmsNew").expect("job is in registry");

    handler.add_to_queue(job);
    let err = handler
        .execute()
        .await
        .expect_err("missing KUmsNew account is rejected");

    assert_eq!(err.kind(), hbci4rust::HbciErrorKind::InvalidArgument);
    assert_eq!(
        err.message(),
        "KUmsNew requires my.iban, my.number, or a passport account for the current KUmsNew7 tracer renderer"
    );
    assert_eq!(replay.requests().expect("requests").len(), 0);
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
    let passport = passport_with_cached_pin(PinTanPassportData {
        accounts: vec![giro_account()],
        ..signed_pintan_data()
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
        body.contains("HKSAL:3:7+DE02123456780000000000:MARKDEF1100:0001234567::280:12345678+N'")
    );
    assert_signed_custom_msg_request(&body, "0", "1", 5);
}

#[tokio::test]
async fn handler_requests_onestep_tan_for_required_signed_segment() {
    let _guard = RUNTIME_CALLBACK_TEST_LOCK.lock().await;
    done().expect("runtime reset");
    let events = Arc::new(Mutex::new(Vec::new()));
    init(
        BTreeMap::<String, String>::new(),
        Arc::new(ScriptedCallback {
            events: events.clone(),
            responses: Arc::new(Mutex::new(VecDeque::from([CallbackResponse::value(
                "987654",
            )]))),
        }),
    )
    .expect("runtime init");

    let passport = passport_with_cached_pin(PinTanPassportData {
        accounts: vec![giro_account()],
        bpd_parameters: BTreeMap::from([
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.segcode".to_owned(),
                "HKSAL".to_owned(),
            ),
            (
                "Params.PinTanPar1.ParPinTan.PinTanGV1.needtan".to_owned(),
                "J".to_owned(),
            ),
            (
                "Params.SaldoPar7.SegHead.code".to_owned(),
                "HISALS".to_owned(),
            ),
        ]),
        ..signed_pintan_data()
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
    let requests = replay.requests().expect("requests");
    let body = String::from_utf8(requests[0].body.clone()).expect("request body is text");

    let sig_tail = fints_segment(&body, "HNSHA");
    let sig_tail_parts = sig_tail.split('+').collect::<Vec<_>>();
    assert_eq!(sig_tail_parts.get(2).copied(), Some(""));
    assert_eq!(sig_tail_parts.get(3).copied(), Some("12345:987654"));

    let events = events.lock().expect("callback event lock");
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].reason, CallbackReason::NeedPtTan);
    assert_eq!(events[0].data_type, CallbackDataType::Text);
    assert_eq!(events[0].message, "Please enter a TAN now");
    assert_eq!(events[0].current_value, None);
    drop(events);
    done().expect("runtime reset");
}

#[tokio::test]
async fn handler_renders_repeated_saldo_requests() {
    let passport = passport_with_cached_pin(signed_pintan_data());
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

    assert_signed_custom_msg_request(&body, "0", "1", 6);
    assert!(body.contains("HKSAL:3:7+DE02123456780000000000+N'"));
    assert!(body.contains("HKSAL:4:7+DE02123456780000000001+N'"));
}

#[tokio::test]
async fn handler_renders_saldo_request_all_without_account() {
    let passport = passport_with_cached_pin(signed_pintan_data());
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

    assert_signed_custom_msg_request(&body, "0", "1", 5);
    assert!(body.contains("HKSAL:3:7++J'"));
}

#[tokio::test]
async fn handler_marks_segment_return_errors_as_failed_jobs() {
    let passport = passport_with_cached_pin(signed_pintan_data());
    let replay = ReplayCommClient::new([Ok(custom_msg_response(&[
        "HIRMG:2:2+0010::OK",
        "HIRMS:3:2+9010:3:Saldo abgelehnt",
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
        vec!["0010:OK".to_owned(), "9010:Saldo abgelehnt (3)".to_owned()]
    );
}
