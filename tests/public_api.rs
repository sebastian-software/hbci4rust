use hbci4rust::{
    HbciExecStatus, HbciHandler, HbciReturnValue, JobRegistry, KnownReturncode, Konto,
    PINTAN_JOB_NAMES, PinTanPassport, PinTanPassportData, ReplayCommClient,
    sepa::CAMT_052_001_01_URN,
};

fn public_passport() -> PinTanPassport {
    PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    })
}

fn public_account() -> Konto {
    Konto {
        country: Some("DE".to_owned()),
        blz: Some("12345678".to_owned()),
        number: Some("1234567890".to_owned()),
        bic: Some("TESTDEFFXXX".to_owned()),
        iban: Some("DE02123456780012345678".to_owned()),
        ..Konto::default()
    }
}

#[test]
fn balance_request_migration_shape_uses_crate_root_api() {
    assert!(PINTAN_JOB_NAMES.contains(&"SaldoReq"));
    assert!(JobRegistry::pintan().contains("SaldoReq"));
    assert!(!JobRegistry::pintan().contains("GVTemplate"));

    let replay = ReplayCommClient::default();
    let mut handler = HbciHandler::with_comm("300", public_passport(), replay);
    let mut job = handler.new_job("SaldoReq").expect("job is in registry");

    job.set_param_account("my", &public_account());
    handler
        .try_add_to_queue(job)
        .expect("balance request constraints resolve");

    let queued = handler.queued_jobs();
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0].name(), "SaldoReq");
    assert_eq!(queued[0].param("my.iban"), Some("DE02123456780012345678"));
}

#[test]
fn error_reporting_migration_shape_uses_crate_root_status_api() {
    let status = HbciExecStatus {
        success: false,
        global_return_values: vec![HbciReturnValue::new("0010", "Dialog OK")],
        segment_return_values: vec![HbciReturnValue::new("9942", "PIN falsch")],
        ..HbciExecStatus::default()
    };

    assert!(!status.success);
    assert!(!status.is_ok());
    assert!(status.message_status().is_ok());
    assert_eq!(status.error_string(), "9942:PIN falsch");
    assert!(status.is_invalid_pin());
    assert_eq!(status.invalid_pin_code().unwrap().code, "9942");
    assert_eq!(
        status
            .return_value_for_code(KnownReturncode::E9942)
            .unwrap()
            .text,
        "PIN falsch"
    );
}

#[test]
fn statement_migration_shapes_use_crate_root_api() {
    let replay = ReplayCommClient::default();
    let mut handler = HbciHandler::with_comm("300", public_passport(), replay);

    let mut mt940 = handler.new_job("KUmsAll").expect("job is in registry");
    mt940.set_param_account("my", &public_account());
    mt940
        .try_set_param_date("startdate", "2026-06-01")
        .expect("startdate is accepted");
    mt940
        .try_set_param_date("enddate", "2026-06-06")
        .expect("enddate is accepted");
    mt940
        .try_set_param_int("maxentries", 25)
        .expect("maxentries is accepted");
    handler
        .try_add_to_queue(mt940)
        .expect("KUmsAll constraints resolve");

    let mut camt = handler.new_job("KUmsAllCamt").expect("job is in registry");
    camt.set_param_account("my", &public_account());
    camt.try_set_param_date("startdate", "2026-06-01")
        .expect("startdate is accepted");
    camt.try_set_param_date("enddate", "2026-06-06")
        .expect("enddate is accepted");
    camt.try_set_param_int("maxentries", 25)
        .expect("maxentries is accepted");
    camt.set_param("offset", "CURSOR");
    handler
        .try_add_to_queue(camt)
        .expect("KUmsAllCamt constraints resolve");

    let queued = handler.queued_jobs();
    assert_eq!(queued.len(), 2);
    assert_eq!(queued[0].name(), "KUmsAll");
    assert_eq!(
        queued[0].lowlevel_param("KUmsZeit7.KTV.iban"),
        Some("DE02123456780012345678")
    );
    assert_eq!(queued[0].lowlevel_param("KUmsZeit7.maxentries"), Some("25"));
    assert_eq!(queued[0].lowlevel_param("KUmsZeit7.allaccounts"), Some("N"));

    assert_eq!(queued[1].name(), "KUmsAllCamt");
    assert_eq!(
        queued[1].lowlevel_param("KUmsZeitCamt1.formats.suppformat"),
        Some(CAMT_052_001_01_URN)
    );
    assert_eq!(
        queued[1].lowlevel_param("KUmsZeitCamt1.offset"),
        Some("CURSOR")
    );
}

#[test]
fn sepa_payment_migration_shapes_generate_public_pain_payloads() {
    let replay = ReplayCommClient::default();
    let mut handler = HbciHandler::with_comm("300", public_passport(), replay);

    let mut transfer = handler.new_job("UebSEPA").expect("job is in registry");
    transfer
        .try_set_param("src.iban", "DE02123456780012345678")
        .expect("source iban is accepted");
    transfer
        .try_set_param("src.bic", "TESTDEFFXXX")
        .expect("source bic is accepted");
    transfer
        .try_set_param("src.name", "Sender Name")
        .expect("source name is accepted");
    transfer
        .try_set_param("dst.iban", "DE99123456780098765432")
        .expect("destination iban is accepted");
    transfer
        .try_set_param("dst.bic", "DEUTDEDB277")
        .expect("destination bic is accepted");
    transfer
        .try_set_param("dst.name", "Receiver Name")
        .expect("destination name is accepted");
    transfer
        .try_set_param("btg.value", "12.30")
        .expect("amount is accepted");
    transfer
        .try_set_param("usage", "Invoice 4711")
        .expect("usage is accepted");
    transfer
        .try_set_param("sepaid", "SEPA-UEB")
        .expect("sepaid is accepted");
    handler
        .try_add_to_queue(transfer)
        .expect("UebSEPA constraints resolve");

    let mut debit = handler.new_job("LastSEPA").expect("job is in registry");
    debit
        .try_set_param("src.iban", "DE02123456780012345678")
        .expect("source iban is accepted");
    debit
        .try_set_param("src.bic", "TESTDEFFXXX")
        .expect("source bic is accepted");
    debit
        .try_set_param("src.name", "Creditor Name")
        .expect("source name is accepted");
    debit
        .try_set_param("dst.iban", "DE99123456780098765432")
        .expect("destination iban is accepted");
    debit
        .try_set_param("dst.bic", "DEUTDEDB277")
        .expect("destination bic is accepted");
    debit
        .try_set_param("dst.name", "Debtor Name")
        .expect("destination name is accepted");
    debit
        .try_set_param("btg.value", "12.30")
        .expect("amount is accepted");
    debit
        .try_set_param("usage", "Direct debit usage")
        .expect("usage is accepted");
    debit
        .try_set_param("sepaid", "SEPA-LAST")
        .expect("sepaid is accepted");
    debit
        .try_set_param("creditorid", "DE98ZZZ09999999999")
        .expect("creditor id is accepted");
    debit
        .try_set_param("mandateid", "MND-123")
        .expect("mandate id is accepted");
    debit
        .try_set_param("manddateofsig", "2026-01-02")
        .expect("mandate date is accepted");
    debit
        .try_set_param("targetdate", "2026-03-15")
        .expect("target date is accepted");
    handler
        .try_add_to_queue(debit)
        .expect("LastSEPA constraints resolve");

    let queued = handler.queued_jobs();
    assert_eq!(queued.len(), 2);

    let transfer_pain = queued[0]
        .lowlevel_param("UebSEPA1.sepapain")
        .expect("UebSEPA generated PAIN");
    assert!(transfer_pain.contains("<pain.001.001.02>"));
    assert!(transfer_pain.contains("<MsgId>SEPA-UEB</MsgId>"));
    assert!(transfer_pain.contains("<Ustrd>Invoice 4711</Ustrd>"));

    let debit_pain = queued[1]
        .lowlevel_param("LastSEPA1.sepapain")
        .expect("LastSEPA generated PAIN");
    assert!(debit_pain.contains("<pain.008.001.01>"));
    assert!(debit_pain.contains("<MsgId>SEPA-LAST</MsgId>"));
    assert!(debit_pain.contains("<OthrId><Id>DE98ZZZ09999999999</Id><IdTp>SEPA</IdTp></OthrId>"));
    assert!(debit_pain.contains("<MndtId>MND-123</MndtId>"));
}
