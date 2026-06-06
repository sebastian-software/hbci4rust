use std::collections::BTreeMap;

use hbci4rust::KnownReturncode;
use hbci4rust::{
    HbciDialogStatus, HbciErrorKind, HbciExecStatus, HbciInstMessage, HbciJobResult, HbciMsgStatus,
    HbciReturnValue, HbciStatus, HbciStatusCode,
};

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
fn inst_message_display_matches_original_shape() {
    let message = HbciInstMessage::new("Wartung", Some("Am Wochenende".to_owned()));

    assert_eq!(message.to_string(), "Wartung: Am Wochenende");
}

#[test]
fn inst_message_display_renders_missing_text_like_java_null() {
    let message = HbciInstMessage::new("Hinweis", None);

    assert_eq!(message.to_string(), "Hinweis: null");
}

#[test]
fn inst_message_from_values_matches_original_keys() {
    let mut values = BTreeMap::new();
    values.insert("KIMsg.betreff".to_owned(), "Wartung".to_owned());
    values.insert("KIMsg.text".to_owned(), "Am Wochenende".to_owned());

    let message = HbciInstMessage::from_values(&values, "KIMsg").expect("inst message parses");

    assert_eq!(message.subject, "Wartung");
    assert_eq!(message.text.as_deref(), Some("Am Wochenende"));
    assert_eq!(message.to_string(), "Wartung: Am Wochenende");
}

#[test]
fn inst_message_from_values_rejects_missing_subject_like_original() {
    let values = BTreeMap::from([("KIMsg.text".to_owned(), "Am Wochenende".to_owned())]);

    let err =
        HbciInstMessage::from_values(&values, "KIMsg").expect_err("missing subject is rejected");

    assert_eq!(err.kind(), HbciErrorKind::Protocol);
    assert_eq!(err.message(), "institute message KIMsg has no subject");
}

#[test]
fn inst_message_collects_counted_values_like_original() {
    let values = BTreeMap::from([
        ("KIMsg.betreff".to_owned(), "Wartung".to_owned()),
        ("KIMsg.text".to_owned(), "Am Wochenende".to_owned()),
        ("KIMsg_2.betreff".to_owned(), "Hinweis".to_owned()),
        ("KIMsg_3.betreff".to_owned(), "Neue App".to_owned()),
        ("KIMsg_3.text".to_owned(), "Bitte aktualisieren".to_owned()),
    ]);

    let messages = HbciInstMessage::collect_from_values(&values, "KIMsg");

    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0].to_string(), "Wartung: Am Wochenende");
    assert_eq!(messages[1].to_string(), "Hinweis: null");
    assert_eq!(messages[2].to_string(), "Neue App: Bitte aktualisieren");
}

#[test]
fn inst_message_collection_stops_at_first_missing_subject_like_original() {
    let values = BTreeMap::from([
        ("KIMsg.betreff".to_owned(), "Wartung".to_owned()),
        ("KIMsg_2.text".to_owned(), "Text ohne Betreff".to_owned()),
        (
            "KIMsg_3.betreff".to_owned(),
            "Wird nicht gelesen".to_owned(),
        ),
    ]);

    let messages = HbciInstMessage::collect_from_values(&values, "KIMsg");

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].subject, "Wartung");
}

#[test]
fn inst_message_collection_returns_empty_when_first_subject_is_absent() {
    let values = BTreeMap::from([("KIMsg_2.betreff".to_owned(), "Später Hinweis".to_owned())]);

    let messages = HbciInstMessage::collect_from_values(&values, "KIMsg");

    assert!(messages.is_empty());
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
fn msg_status_display_and_error_string_match_original_shape() {
    let msg_status = HbciMsgStatus::from_statuses(
        HbciStatus::from_return_values([
            HbciReturnValue::new("3020", "Globalwarnung"),
            HbciReturnValue::new("0010", "Dialog OK"),
        ]),
        HbciStatus::from_return_values([HbciReturnValue::new("9010", "Segmentfehler")]),
    );

    assert_eq!(msg_status.error_string(), "9010:Segmentfehler");
    assert_eq!(
        msg_status.to_string(),
        "3020:Globalwarnung\n0010:Dialog OK\n9010:Segmentfehler"
    );
}

#[test]
fn msg_status_is_ok_uses_only_global_status_like_original() {
    let msg_status = HbciMsgStatus::from_statuses(
        HbciStatus::from_return_values([HbciReturnValue::new("0010", "Dialog OK")]),
        HbciStatus::from_return_values([HbciReturnValue::new("9010", "Segmentfehler")]),
    );

    assert!(msg_status.is_ok());

    let msg_status_with_global_error = HbciMsgStatus::from_statuses(
        HbciStatus::from_return_values([HbciReturnValue::new("9010", "Dialogfehler")]),
        HbciStatus::from_return_values([HbciReturnValue::new("0010", "Segment OK")]),
    );

    assert!(!msg_status_with_global_error.is_ok());
}

#[test]
fn msg_status_uses_global_exceptions_like_original() {
    let mut global_status = HbciStatus::new();
    global_status.add_exception_message("Transportfehler");
    let mut segment_status = HbciStatus::new();
    segment_status.add_exception_message("Segmentdiagnose");
    let msg_status = HbciMsgStatus::from_statuses(global_status, segment_status);

    assert!(msg_status.has_exceptions());
    assert_eq!(
        msg_status.error_string(),
        "Transportfehler\nSegmentdiagnose"
    );
}

#[test]
fn msg_status_searches_return_codes_and_invalid_pin_like_original() {
    let msg_status = HbciMsgStatus::from_statuses(
        HbciStatus::from_return_values([
            HbciReturnValue::new("3920", "Globale TAN-Liste"),
            HbciReturnValue::new("9930", "PIN gesperrt"),
        ]),
        HbciStatus::from_return_values([
            HbciReturnValue::new("3920", "Segment-TAN-Liste"),
            HbciReturnValue::new("9942", "PIN falsch"),
        ]),
    );

    let tan_values = msg_status.return_values_for_code(KnownReturncode::W3920);

    assert_eq!(tan_values.len(), 2);
    assert_eq!(tan_values[0].text, "Globale TAN-Liste");
    assert_eq!(tan_values[1].text, "Segment-TAN-Liste");
    assert_eq!(
        msg_status
            .return_value_for_code(KnownReturncode::W3920)
            .unwrap()
            .text,
        "Globale TAN-Liste"
    );
    assert!(msg_status.is_invalid_pin());
    assert_eq!(msg_status.invalid_pin_code().unwrap().code, "9930");
}

#[test]
fn dialog_status_is_ok_requires_init_messages_and_end_like_original() {
    let mut dialog_status = HbciDialogStatus::new();
    dialog_status.set_init_status(ok_msg_status("Init OK"));
    dialog_status.set_message_statuses([msg_status_with_segment_error(
        "Nutzdaten OK",
        "Segmentfehler",
    )]);
    dialog_status.set_end_status(ok_msg_status("Ende OK"));

    assert!(dialog_status.is_ok());

    let mut missing_end = HbciDialogStatus::new();
    missing_end.set_init_status(ok_msg_status("Init OK"));

    assert!(!missing_end.is_ok());

    dialog_status.set_message_statuses([global_error_msg_status("Dialogfehler")]);

    assert!(!dialog_status.is_ok());
}

#[test]
fn dialog_status_error_string_joins_parts_without_original_labels() {
    let mut dialog_status = HbciDialogStatus::new();
    dialog_status.set_init_status(global_error_msg_status("Initfehler"));
    dialog_status.set_message_statuses([msg_status_with_segment_error(
        "Nutzdaten OK",
        "Segmentfehler",
    )]);
    dialog_status.set_end_status(global_error_msg_status("Endefehler"));

    assert_eq!(
        dialog_status.error_string(),
        "9010:Initfehler\n9010:Segmentfehler\n9010:Endefehler"
    );
}

#[test]
fn dialog_status_has_exceptions_across_message_parts_like_original() {
    let mut message_global = HbciStatus::new();
    message_global.add_exception_message("Transportfehler");
    let message_status = HbciMsgStatus::from_statuses(message_global, HbciStatus::new());
    let mut dialog_status = HbciDialogStatus::new();
    dialog_status.set_init_status(ok_msg_status("Init OK"));
    dialog_status.set_message_statuses([message_status]);
    dialog_status.set_end_status(ok_msg_status("Ende OK"));

    assert!(dialog_status.has_exceptions());
}

#[test]
fn dialog_status_display_matches_original_sections() {
    let mut dialog_status = HbciDialogStatus::new();
    dialog_status.set_init_status(ok_msg_status("Init OK"));
    dialog_status.set_message_statuses([msg_status_with_segment_error(
        "Nutzdaten OK",
        "Segmentfehler",
    )]);

    assert_eq!(
        dialog_status.to_string(),
        "DIALOG-INIT:\n0010:Init OK\nDIALOG-MSG #1:\n0010:Nutzdaten OK\n9010:Segmentfehler\nDIALOG-END:\n(not status information available)"
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

    let message_status = exec_status.message_status();
    assert_eq!(
        message_status.global_status.status_code(),
        HbciStatusCode::Ok
    );
    assert_eq!(
        message_status.segment_status.status_code(),
        HbciStatusCode::Error
    );

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
fn exec_status_collects_customer_ids_from_dialogs_and_exceptions_like_original() {
    let mut exec_status = HbciExecStatus::default();
    exec_status.add_dialog_status("cust-b", Some(ok_dialog_status()));
    exec_status.add_exception_message("cust-a", "Planungsfehler");

    assert_eq!(exec_status.customer_ids(), vec!["cust-a", "cust-b"]);
    assert_eq!(exec_status.dialog_status_list().len(), 1);
    assert!(exec_status.dialog_status("cust-b").unwrap().is_ok());
    assert_eq!(
        exec_status.exception_messages("cust-a").unwrap()[0],
        "Planungsfehler"
    );

    exec_status.add_dialog_status("cust-b", None);

    assert!(exec_status.dialog_status("cust-b").is_none());
}

#[test]
fn exec_status_error_string_groups_multiple_customers_like_original() {
    let mut exec_status = HbciExecStatus::default();
    exec_status.add_exception_message("cust-a", "Planungsfehler");
    exec_status.add_dialog_status("cust-a", Some(dialog_status_with_init_error("Initfehler")));
    exec_status.add_dialog_status("cust-b", Some(dialog_status_with_end_error("Endefehler")));
    exec_status.add_dialog_status("cust-c", Some(ok_dialog_status()));

    assert_eq!(
        exec_status.error_string(),
        "Dialog for 'cust-a':\nPlanungsfehler\n9010:Initfehler\nDialog for 'cust-b':\n9010:Endefehler"
    );
}

#[test]
fn exec_status_display_for_customer_and_all_dialogs_matches_original_shape() {
    let mut exec_status = HbciExecStatus::default();
    exec_status.add_exception_message("cust-a", "Planungsfehler");
    exec_status.add_dialog_status("cust-a", Some(ok_dialog_status()));

    assert_eq!(
        exec_status.to_string_for_customer("cust-a"),
        "Planungsfehler\nDIALOG-INIT:\n0010:Init OK\nDIALOG-END:\n0010:Ende OK"
    );
    assert_eq!(
        exec_status.to_string(),
        "Dialog for 'cust-a':\nPlanungsfehler\nDIALOG-INIT:\n0010:Init OK\nDIALOG-END:\n0010:Ende OK"
    );
}

#[test]
fn exec_status_is_ok_uses_dialog_status_and_exceptions_like_original() {
    let mut exec_status = HbciExecStatus::default();
    exec_status.add_dialog_status("cust-a", Some(ok_dialog_status()));

    assert!(exec_status.is_ok_for_customer("cust-a"));
    assert!(exec_status.is_ok());
    assert!(!exec_status.is_ok_for_customer("missing"));

    exec_status.add_exception_message("cust-a", "Planungsfehler");

    assert!(!exec_status.is_ok_for_customer("cust-a"));
    assert!(!exec_status.is_ok());
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

#[test]
fn job_result_ret_helpers_count_only_job_status_like_original() {
    let job = HbciJobResult {
        global_return_values: vec![HbciReturnValue::new("0010", "Global OK")],
        return_values: vec![
            HbciReturnValue::new("3020", "Jobwarnung"),
            HbciReturnValue::new("0020", "Job OK"),
        ],
        ..job_result("SaldoReq", Vec::new())
    };

    assert_eq!(job.ret_number(), 2);
    assert_eq!(job.ret_value(0).unwrap().text, "Jobwarnung");
    assert_eq!(job.ret_value(1).unwrap().text, "Job OK");
    assert!(job.ret_value(2).is_none());
    assert_eq!(job.global_status().successes()[0].text, "Global OK");
    assert_eq!(job.job_status().warnings()[0].text, "Jobwarnung");
}

#[test]
fn job_result_is_ok_uses_stored_global_and_job_status_like_original() {
    let ok_job_with_global_ok = HbciJobResult {
        global_return_values: vec![HbciReturnValue::new("0010", "Global OK")],
        ..job_result("SaldoReq", Vec::new())
    };
    let ok_job_with_job_warning = HbciJobResult {
        return_values: vec![HbciReturnValue::new("3020", "Jobwarnung")],
        ..job_result("SaldoReq", Vec::new())
    };
    let unknown_job = job_result("SaldoReq", Vec::new());
    let global_error = HbciJobResult {
        global_return_values: vec![HbciReturnValue::new("9010", "Globalfehler")],
        return_values: vec![HbciReturnValue::new("0020", "Job OK")],
        ..job_result("SaldoReq", Vec::new())
    };
    let job_error = HbciJobResult {
        global_return_values: vec![HbciReturnValue::new("0010", "Global OK")],
        return_values: vec![HbciReturnValue::new("9010", "Jobfehler")],
        ..job_result("SaldoReq", Vec::new())
    };

    assert!(ok_job_with_global_ok.is_ok());
    assert!(ok_job_with_job_warning.is_ok());
    assert!(!unknown_job.is_ok());
    assert!(!global_error.is_ok());
    assert!(!job_error.is_ok());
}

#[test]
fn job_result_result_data_helpers_match_original_basic_properties() {
    let mut job = job_result("SaldoReq", Vec::new());
    job.store_result("content.balance", Some("123.45"));
    job.store_result("basic.dialogid", Some("DIALOG1"));
    job.store_result("basic.msgnum", Some("2"));
    job.store_result("basic.segnum", Some("3"));
    job.store_result("ignored", None::<String>);

    assert_eq!(job.dialog_id(), Some("DIALOG1"));
    assert_eq!(job.msg_num(), Some("2"));
    assert_eq!(job.seg_num(), Some("3"));
    assert_eq!(job.job_id_for_date("20260606"), "20260606/DIALOG1/2/3");
    assert!(!job.result_data.contains_key("ignored"));
    assert_eq!(
        job.to_string(),
        "basic.dialogid = DIALOG1\nbasic.msgnum = 2\nbasic.segnum = 3\ncontent.balance = 123.45"
    );
}

#[test]
fn job_result_job_id_uses_java_null_shape_for_missing_basic_properties() {
    let job = job_result("SaldoReq", Vec::new());

    assert_eq!(job.dialog_id(), None);
    assert_eq!(job.msg_num(), None);
    assert_eq!(job.seg_num(), None);
    assert_eq!(job.job_id_for_date("20260606"), "20260606/null/null/null");
    assert_eq!(job.to_string(), "");
}

fn job_result(name: &str, return_values: Vec<HbciReturnValue>) -> HbciJobResult {
    HbciJobResult {
        job_name: name.to_owned(),
        success: false,
        raw_response: None,
        result_data: BTreeMap::new(),
        global_return_values: Vec::new(),
        return_values,
        result: None,
    }
}

fn ok_msg_status(text: &str) -> HbciMsgStatus {
    HbciMsgStatus::from_statuses(
        HbciStatus::from_return_values([HbciReturnValue::new("0010", text)]),
        HbciStatus::new(),
    )
}

fn global_error_msg_status(text: &str) -> HbciMsgStatus {
    HbciMsgStatus::from_statuses(
        HbciStatus::from_return_values([HbciReturnValue::new("9010", text)]),
        HbciStatus::new(),
    )
}

fn msg_status_with_segment_error(global_text: &str, segment_error_text: &str) -> HbciMsgStatus {
    HbciMsgStatus::from_statuses(
        HbciStatus::from_return_values([HbciReturnValue::new("0010", global_text)]),
        HbciStatus::from_return_values([HbciReturnValue::new("9010", segment_error_text)]),
    )
}

fn ok_dialog_status() -> HbciDialogStatus {
    let mut dialog_status = HbciDialogStatus::new();
    dialog_status.set_init_status(ok_msg_status("Init OK"));
    dialog_status.set_end_status(ok_msg_status("Ende OK"));
    dialog_status
}

fn dialog_status_with_init_error(text: &str) -> HbciDialogStatus {
    let mut dialog_status = ok_dialog_status();
    dialog_status.set_init_status(global_error_msg_status(text));
    dialog_status
}

fn dialog_status_with_end_error(text: &str) -> HbciDialogStatus {
    let mut dialog_status = ok_dialog_status();
    dialog_status.set_end_status(global_error_msg_status(text));
    dialog_status
}
