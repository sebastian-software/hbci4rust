use hbci4rust::{GvrKUms, GvrKUmsLine, GvrSaldoReq, GvrSaldoReqInfo, Konto, Limit, Saldo, Value};

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

#[test]
fn kums_mt940_shell_parses_block_header_like_original() {
    let mut result = GvrKUms::new();
    result.append_mt940_data(concat!(
        "\r\n:20:STARTUMSE",
        "\r\n:25:12030000/1019815776EUR",
        "\r\n:28C:00000/002",
        "\r\n:60F:C260601EUR100,00",
        "\r\n:62M:D260606EUR12 3,45",
        "\r\n-"
    ));

    {
        let days = result.get_data_per_day();
        assert_eq!(days.len(), 1);
        let day = &days[0];
        assert_eq!(day.my.blz.as_deref(), Some("12030000"));
        assert_eq!(day.my.number.as_deref(), Some("1019815776"));
        assert_eq!(day.my.iban.as_deref(), Some(""));
        assert_eq!(day.my.curr.as_deref(), Some("EUR"));
        assert_eq!(day.counter.as_deref(), Some("00000/002"));
        assert_eq!(day.start_type, 'F');
        assert_eq!(
            day.start.as_ref().map(ToString::to_string).as_deref(),
            Some("260601 100.00 EUR")
        );
        assert_eq!(day.end_type, 'M');
        assert_eq!(
            day.end.as_ref().map(ToString::to_string).as_deref(),
            Some("260606 -123.45 EUR")
        );
        assert!(day.lines.is_empty());
    }

    assert_eq!(result.rest_mt940, "");
    assert!(result.get_flat_data().is_empty());
}

#[test]
fn kums_mt940_shell_splits_concatenated_blocks_like_original() {
    let mut result = GvrKUms::new();
    result.append_mt940_data(concat!(
        "\r\n:20:FIRST",
        "\r\n:25:DE02123456780000000000",
        "\r\n:28C:001/001",
        "\r\n-",
        "\r\n:20:SECOND",
        "\r\n:25:12030000/1019815776",
        "\r\n:28C:002/001",
        "\r\n-"
    ));

    let days = result.get_data_per_day();
    assert_eq!(days.len(), 2);
    assert_eq!(days[0].my.iban.as_deref(), Some("DE02123456780000000000"));
    assert_eq!(days[0].my.blz.as_deref(), Some(""));
    assert_eq!(days[0].my.number.as_deref(), Some(""));
    assert_eq!(days[0].counter.as_deref(), Some("001/001"));
    assert_eq!(days[1].my.blz.as_deref(), Some("12030000"));
    assert_eq!(days[1].my.number.as_deref(), Some("1019815776"));
    assert_eq!(days[1].my.curr.as_deref(), Some(""));
    assert_eq!(days[1].counter.as_deref(), Some("002/001"));
}

#[test]
fn kums_mt942_shell_keeps_unbooked_data_separate_like_original() {
    let mut result = GvrKUms::new();
    result.append_mt942_data(concat!(
        "\r\n:20:STARTUMSV",
        "\r\n:25:12030000/1019815776",
        "\r\n:28C:003/001",
        "\r\n:62F:C260606EUR1,00",
        "\r\n-"
    ));

    assert!(result.get_data_per_day().is_empty());
    {
        let unbooked = result.get_data_per_day_unbooked();
        assert_eq!(unbooked.len(), 1);
        assert_eq!(unbooked[0].counter.as_deref(), Some("003/001"));
        assert_eq!(
            unbooked[0].end.as_ref().map(ToString::to_string).as_deref(),
            Some("260606 1.00 EUR")
        );
    }
    assert_eq!(result.rest_mt942, "");
    assert!(result.get_flat_data_unbooked().is_empty());
}

#[test]
fn kums_line_add_usage_skips_absent_values_like_original() {
    let mut line = GvrKUmsLine::default();

    line.add_usage(None);
    line.add_usage(Some("SVWZ+Invoice 123".to_owned()));

    assert_eq!(line.usage, vec!["SVWZ+Invoice 123"]);
    assert!(!line.is_sepa);
    assert!(!line.is_camt);
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
