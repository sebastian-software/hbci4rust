use hbci4rust::KnownReturncode;
use hbci4rust::{HbciExecStatus, HbciJobResult, HbciReturnValue, HbciStatus, HbciStatusCode};

#[test]
fn return_value_display_matches_original_shape() {
    let mut value = HbciReturnValue::new("3020", "Hinweis");
    value.params = vec!["alpha".to_owned(), "beta".to_owned()];
    value.segment_ref = Some("4".to_owned());
    value.data_ref = Some("2".to_owned());
    value.element = Some("CustomMsgRes.GVRes.SaldoRes7=ok".to_owned());

    assert_eq!(
        value.to_string(),
        "3020:Hinweis p:alpha p:beta (4:2: CustomMsgRes.GVRes.SaldoRes7=ok)"
    );
    assert_eq!(value.message(), value.to_string());
}

#[test]
fn return_value_display_omits_absent_references_like_original() {
    assert_eq!(HbciReturnValue::new("0010", "OK").to_string(), "0010:OK");

    let mut value = HbciReturnValue::new("9010", "Fehler");
    value.segment_ref = Some("7".to_owned());

    assert_eq!(value.to_string(), "9010:Fehler (7)");
}

#[test]
fn return_value_display_omits_element_without_segment_ref_like_original() {
    let mut value = HbciReturnValue::new("3020", "Hinweis");
    value.element = Some("CustomMsgRes.GVRes.SaldoRes7=ignored".to_owned());

    assert_eq!(value.to_string(), "3020:Hinweis");
}

#[test]
fn return_value_equality_matches_original_compared_fields() {
    let mut left = HbciReturnValue::new("3020", "Hinweis");
    left.segment_ref = Some("4".to_owned());
    left.data_ref = Some("2".to_owned());
    left.params = vec!["alpha".to_owned()];
    left.element = Some("CustomMsgRes.GVRes.SaldoRes7=left".to_owned());

    let mut right = left.clone();
    right.params = vec!["beta".to_owned(), "gamma".to_owned()];
    right.element = Some("CustomMsgRes.GVRes.SaldoRes7=right".to_owned());

    assert_eq!(left, right);

    let mut different_ref = left.clone();
    different_ref.data_ref = Some("3".to_owned());
    assert_ne!(left, different_ref);

    let mut different_text = left.clone();
    different_text.text = "Anderer Hinweis".to_owned();
    assert_ne!(left, different_text);
}

#[test]
fn status_groups_return_values_like_original() {
    let mut status = HbciStatus::new();
    status.add_return_value(HbciReturnValue::new("3020", "Warnung"));
    status.add_return_value(HbciReturnValue::new("0010", "OK"));
    status.add_return_value(HbciReturnValue::new("9010", "Fehler"));
    status.add_exception_message("Transportfehler");

    assert!(status.has_exceptions());
    assert!(status.has_errors());
    assert!(status.has_warnings());
    assert!(status.has_success());
    assert_eq!(status.status_code(), HbciStatusCode::Error);
    assert!(!status.is_ok());
    assert_eq!(status.errors()[0].code, "9010");
    assert_eq!(status.warnings()[0].code, "3020");
    assert_eq!(status.successes()[0].code, "0010");
    assert_eq!(status.error_string(), "Transportfehler\n9010:Fehler");
    assert_eq!(
        status.to_string(),
        "Transportfehler\n9010:Fehler\n3020:Warnung\n0010:OK"
    );
}

#[test]
fn status_code_matches_original_ok_unknown_error_order() {
    assert_eq!(HbciStatus::new().status_code(), HbciStatusCode::Unknown);
    assert_eq!(
        HbciStatusCode::Ok.original_code(),
        HbciStatusCode::STATUS_OK
    );
    assert_eq!(
        HbciStatusCode::Unknown.original_code(),
        HbciStatusCode::STATUS_UNKNOWN
    );
    assert_eq!(
        HbciStatusCode::Error.original_code(),
        HbciStatusCode::STATUS_ERR
    );

    let mut warning_status = HbciStatus::new();
    warning_status.add_return_value(HbciReturnValue::new("3020", "Warnung"));
    assert_eq!(warning_status.status_code(), HbciStatusCode::Ok);
    assert!(warning_status.is_ok());
}

#[test]
fn status_searches_return_values_for_known_code_like_original() {
    let status = HbciStatus::from_return_values([
        HbciReturnValue::new("3920", "TAN-Verfahren"),
        HbciReturnValue::new("3920", "Weiteres TAN-Verfahren"),
        HbciReturnValue::new("0010", "OK"),
    ]);

    let values = status.return_values_for_code(KnownReturncode::W3920);

    assert_eq!(values.len(), 2);
    assert_eq!(values[0].text, "TAN-Verfahren");
    assert_eq!(values[1].text, "Weiteres TAN-Verfahren");
    assert_eq!(
        status
            .return_value_for_code(KnownReturncode::W3920)
            .unwrap()
            .text,
        "TAN-Verfahren"
    );
    assert!(
        status
            .return_value_for_code(KnownReturncode::E9391)
            .is_none()
    );
}

#[test]
fn exec_and_job_status_helpers_group_existing_return_values() {
    let exec_status = HbciExecStatus {
        global_return_values: vec![HbciReturnValue::new("0010", "Dialog OK")],
        segment_return_values: vec![HbciReturnValue::new("9010", "Segmentfehler")],
        job_results: vec![job_result(
            "SaldoReq",
            vec![HbciReturnValue::new("3020", "Warnung")],
        )],
        ..HbciExecStatus::default()
    };

    let global_status = exec_status.global_status();
    assert_eq!(global_status.status_code(), HbciStatusCode::Ok);
    assert_eq!(global_status.successes()[0].text, "Dialog OK");

    let segment_status = exec_status.segment_status();
    assert_eq!(segment_status.status_code(), HbciStatusCode::Error);
    assert_eq!(segment_status.errors()[0].text, "Segmentfehler");

    let job = &exec_status.job_results[0];
    assert_eq!(job.job_status().status_code(), HbciStatusCode::Ok);
    assert!(job.is_ok_with_global_status(&global_status));
}

#[test]
fn exec_status_error_string_and_display_match_message_status_shape() {
    let exec_status = HbciExecStatus {
        global_return_values: vec![
            HbciReturnValue::new("3020", "Globalwarnung"),
            HbciReturnValue::new("0010", "Dialog OK"),
        ],
        segment_return_values: vec![HbciReturnValue::new("9010", "Segmentfehler")],
        ..HbciExecStatus::default()
    };

    assert_eq!(exec_status.error_string(), "9010:Segmentfehler");
    assert_eq!(
        exec_status.to_string(),
        "3020:Globalwarnung\n0010:Dialog OK\n9010:Segmentfehler"
    );
}

#[test]
fn exec_status_error_string_trims_empty_global_or_segment_status() {
    let exec_status = HbciExecStatus {
        segment_return_values: vec![HbciReturnValue::new("9010", "Segmentfehler")],
        ..HbciExecStatus::default()
    };

    assert_eq!(exec_status.error_string(), "9010:Segmentfehler");
    assert_eq!(exec_status.to_string(), "9010:Segmentfehler");
}

#[test]
fn known_returncode_auth_fail_list_matches_original() {
    let codes = KnownReturncode::LIST_AUTH_FAIL
        .iter()
        .map(|code| code.code())
        .collect::<Vec<_>>();

    assert_eq!(codes, vec!["9340", "9930", "9931", "9942"]);
    assert_eq!(
        KnownReturncode::find("9930", &KnownReturncode::LIST_AUTH_FAIL),
        Some(KnownReturncode::E9930)
    );
    assert!(KnownReturncode::contains(
        "9942",
        &KnownReturncode::LIST_AUTH_FAIL
    ));
    assert!(!KnownReturncode::contains(
        "",
        &KnownReturncode::LIST_AUTH_FAIL
    ));
    assert!(!KnownReturncode::contains(
        "3920",
        &KnownReturncode::LIST_AUTH_FAIL
    ));
}

#[test]
fn exec_status_searches_known_return_codes_across_global_and_segment_values() {
    let exec_status = HbciExecStatus {
        global_return_values: vec![
            HbciReturnValue::new("3920", "Globale TAN-Liste"),
            HbciReturnValue::new("0010", "OK"),
        ],
        segment_return_values: vec![HbciReturnValue::new("3920", "Segment-TAN-Liste")],
        ..HbciExecStatus::default()
    };

    let values = exec_status.return_values_for_code(KnownReturncode::W3920);

    assert_eq!(values.len(), 2);
    assert_eq!(values[0].text, "Globale TAN-Liste");
    assert_eq!(values[1].text, "Segment-TAN-Liste");
    assert_eq!(
        exec_status
            .return_value_for_code(KnownReturncode::W3920)
            .unwrap()
            .text,
        "Globale TAN-Liste"
    );
    assert!(
        exec_status
            .return_value_for_code(KnownReturncode::E9391)
            .is_none()
    );
}

#[test]
fn exec_status_detects_invalid_pin_code_like_message_status() {
    let exec_status = HbciExecStatus {
        global_return_values: vec![
            HbciReturnValue::new("9010", "Anderer Fehler"),
            HbciReturnValue::new("9930", "PIN gesperrt"),
        ],
        segment_return_values: vec![HbciReturnValue::new("9942", "PIN falsch")],
        ..HbciExecStatus::default()
    };

    assert!(exec_status.is_invalid_pin());
    assert_eq!(exec_status.invalid_pin_code().unwrap().code, "9930");
}

#[test]
fn exec_status_ignores_non_auth_fail_errors_for_invalid_pin() {
    let exec_status = HbciExecStatus {
        global_return_values: vec![HbciReturnValue::new("9010", "Anderer Fehler")],
        segment_return_values: vec![HbciReturnValue::new("9000", "Noch ein Fehler")],
        ..HbciExecStatus::default()
    };

    assert!(!exec_status.is_invalid_pin());
    assert!(exec_status.invalid_pin_code().is_none());
}

#[test]
fn job_status_ok_with_global_status_matches_original_rule() {
    let ok_global = HbciStatus::from_return_values([HbciReturnValue::new("0010", "OK")]);
    let unknown_global = HbciStatus::new();
    let error_global = HbciStatus::from_return_values([HbciReturnValue::new("9010", "Fehler")]);

    let unknown_job = job_result("SaldoReq", Vec::new());
    let warning_job = job_result("SaldoReq", vec![HbciReturnValue::new("3020", "Warnung")]);
    let error_job = job_result("SaldoReq", vec![HbciReturnValue::new("9010", "Fehler")]);

    assert!(unknown_job.is_ok_with_global_status(&ok_global));
    assert!(warning_job.is_ok_with_global_status(&unknown_global));
    assert!(!unknown_job.is_ok_with_global_status(&unknown_global));
    assert!(!warning_job.is_ok_with_global_status(&error_global));
    assert!(!error_job.is_ok_with_global_status(&ok_global));
}

fn job_result(name: &str, return_values: Vec<HbciReturnValue>) -> HbciJobResult {
    HbciJobResult {
        job_name: name.to_owned(),
        success: false,
        raw_response: None,
        return_values,
        result: None,
    }
}
