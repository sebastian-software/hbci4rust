use hbci4rust::{
    HbciHandler, JobRegistry, Konto, PINTAN_JOB_NAMES, PinTanPassport, PinTanPassportData,
    ReplayCommClient,
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
