use hbci4rust::{Limit, Saldo, Value};

#[test]
fn value_display_matches_original_shape_for_simple_amounts() {
    assert_eq!(
        Value {
            value: "12.3".to_owned(),
            curr: Some("EUR".to_owned()),
        }
        .to_string(),
        "12.30 EUR"
    );
    assert_eq!(
        Value {
            value: "1 234.5".to_owned(),
            curr: None,
        }
        .to_string(),
        "1234.50 null"
    );
    assert_eq!(
        Value {
            value: "-000.5".to_owned(),
            curr: Some("EUR".to_owned()),
        }
        .to_string(),
        "-0.50 EUR"
    );
}

#[test]
fn value_display_preserves_unparseable_tracer_amounts() {
    assert_eq!(
        Value {
            value: "1.234".to_owned(),
            curr: Some("EUR".to_owned()),
        }
        .to_string(),
        "1.234 EUR"
    );
}

#[test]
fn saldo_display_uses_stored_timestamp_parts_and_value() {
    assert_eq!(
        Saldo {
            value: Value {
                value: "-123.4".to_owned(),
                curr: Some("EUR".to_owned()),
            },
            date: Some("2026-06-05".to_owned()),
            time: Some("07:08:09".to_owned()),
        }
        .to_string(),
        "2026-06-05 07:08:09 -123.40 EUR"
    );
}

#[test]
fn limit_display_matches_original_labels() {
    assert_eq!(
        Limit {
            limit_type: Limit::TYPE_DAILY.to_owned(),
            value: Some(Value {
                value: "1000".to_owned(),
                curr: Some("EUR".to_owned()),
            }),
            days: None,
        }
        .to_string(),
        "Tageslimit: 1000.00 EUR"
    );
    assert_eq!(
        Limit {
            limit_type: Limit::TYPE_TIME.to_owned(),
            value: Some(Value {
                value: "50".to_owned(),
                curr: Some("EUR".to_owned()),
            }),
            days: Some(14),
        }
        .to_string(),
        "Zeitliches Limit (14 Tage): 50.00 EUR"
    );
    assert_eq!(
        Limit {
            limit_type: Limit::TYPE_SINGLE.to_owned(),
            value: None,
            days: None,
        }
        .to_string(),
        "Einzellimit: null"
    );
}
