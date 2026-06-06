use hbci4rust::{HbciReturnValue, HbciStatus, HbciStatusCode};

#[test]
fn return_value_display_matches_original_shape() {
    let mut value = HbciReturnValue::new("3020", "Hinweis");
    value.params = vec!["alpha".to_owned(), "beta".to_owned()];
    value.segment_ref = Some("4".to_owned());
    value.data_ref = Some("2".to_owned());

    assert_eq!(value.to_string(), "3020:Hinweis p:alpha p:beta (4:2)");
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
