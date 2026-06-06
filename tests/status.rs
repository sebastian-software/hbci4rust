use hbci4rust::HbciReturnValue;

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
