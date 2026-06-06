use hbci4rust::{GvrSaldoReq, GvrSaldoReqInfo, Konto, Limit, Saldo, Value};

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

#[test]
fn saldo_result_info_display_matches_original_line_order() {
    assert_eq!(
        saldo_info_all_fields().to_string(),
        concat!(
            "Konto: 0001234567 (EUR)\n",
            "  Gebucht: 2026-06-05 07:08:09 123.45 EUR\n",
            "  Pending: 2026-06-05 -1.23 EUR\n",
            "  Kredit: 1000.00 EUR\n",
            "  Verfügbar: 900.00 EUR\n",
            "  Benutzt: 100.00 EUR"
        )
    );
}

#[test]
fn saldo_result_display_joins_entries_without_trailing_newline() {
    let result = GvrSaldoReq {
        entries: vec![saldo_info_all_fields(), saldo_info_minimal("0007654321")],
    };

    assert_eq!(
        result.to_string(),
        concat!(
            "Konto: 0001234567 (EUR)\n",
            "  Gebucht: 2026-06-05 07:08:09 123.45 EUR\n",
            "  Pending: 2026-06-05 -1.23 EUR\n",
            "  Kredit: 1000.00 EUR\n",
            "  Verfügbar: 900.00 EUR\n",
            "  Benutzt: 100.00 EUR\n",
            "Konto: 0007654321 (EUR)\n",
            "  Gebucht: 2026-06-05 10.00 EUR"
        )
    );
    assert_eq!(GvrSaldoReq::default().to_string(), "");
}

fn saldo_info_all_fields() -> GvrSaldoReqInfo {
    GvrSaldoReqInfo {
        konto: test_account("0001234567"),
        ready: test_saldo("123.45", Some("07:08:09")),
        unready: Some(Saldo {
            value: Value {
                value: "-1.23".to_owned(),
                curr: Some("EUR".to_owned()),
            },
            date: Some("2026-06-05".to_owned()),
            time: None,
        }),
        kredit: Some(test_value("1000")),
        available: Some(test_value("900")),
        used: Some(test_value("100")),
    }
}

fn saldo_info_minimal(number: &str) -> GvrSaldoReqInfo {
    GvrSaldoReqInfo {
        konto: test_account(number),
        ready: test_saldo("10", None),
        unready: None,
        kredit: None,
        available: None,
        used: None,
    }
}

fn test_account(number: &str) -> Konto {
    Konto {
        number: Some(number.to_owned()),
        ..Konto::default()
    }
}

fn test_saldo(value: &str, time: Option<&str>) -> Saldo {
    Saldo {
        value: test_value(value),
        date: Some("2026-06-05".to_owned()),
        time: time.map(str::to_owned),
    }
}

fn test_value(value: &str) -> Value {
    Value {
        value: value.to_owned(),
        curr: Some("EUR".to_owned()),
    }
}
