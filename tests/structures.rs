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
fn kums_mt940_parser_reads_lines_with_explicit_booking_date_like_original() {
    let mut result = GvrKUms::new();
    result.append_mt940_data(concat!(
        "\r\n:20:STARTUMS",
        "\r\n:25:12345678/1234567890",
        "\r\n:28C:1",
        "\r\n:60F:C230209EUR100,00",
        "\r\n:61:2302090209CR2,00NTRF2023-02-09-08.37.18.054696",
        "\r\n:86:152?00GUTSCHRIFT UEBERWEISUNG?109245?20Test 1?32Max Mustermann?34000",
        "\r\n:61:2302090209CR1,00NTRF2023-02-09-08.37.18.552784",
        "\r\n:86:152?00GUTSCHRIFT UEBERWEISUNG?109245?20Test 2?32Max Mustermann?34000",
        "\r\n:62F:C230209EUR103,00",
        "\r\n-"
    ));

    let lines = result.get_flat_data();
    assert_eq!(lines.len(), 2);
    assert_eq!(lines[0].valuta.as_deref(), Some("230209"));
    assert_eq!(lines[0].bdate.as_deref(), Some("230209"));
    assert_eq!(
        lines[0].value.as_ref().map(ToString::to_string).as_deref(),
        Some("2.00 EUR")
    );
    assert_eq!(
        lines[0].saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("230209 102.00 EUR")
    );
    assert_eq!(
        lines[0].customerref.as_deref(),
        Some("2023-02-09-08.37.18.054696")
    );
    assert_eq!(lines[0].instref.as_deref(), Some(""));
    assert_eq!(lines[0].gvcode.as_deref(), Some("152"));
    assert!(lines[0].is_sepa);
    assert_eq!(lines[0].text.as_deref(), Some("GUTSCHRIFT UEBERWEISUNG"));
    assert_eq!(lines[0].primanota.as_deref(), Some("9245"));
    assert_eq!(lines[0].usage, vec!["Test 1"]);
    assert_eq!(
        lines[0]
            .other
            .as_ref()
            .and_then(|konto| konto.name.as_deref()),
        Some("Max Mustermann")
    );
    assert_eq!(lines[0].addkey.as_deref(), Some("000"));
    assert_eq!(
        lines[1].value.as_ref().map(ToString::to_string).as_deref(),
        Some("1.00 EUR")
    );
    assert_eq!(
        lines[1].saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("230209 103.00 EUR")
    );
}

#[test]
fn kums_mt940_parser_uses_start_date_when_booking_date_is_missing_like_original() {
    let mut result = GvrKUms::new();
    result.append_mt940_data(concat!(
        "\r\n:20:STARTUMS",
        "\r\n:25:12345678/1234567890",
        "\r\n:60F:C230209EUR100,00",
        "\r\n:61:230209CR2,00NTRFNOBOOKINGDATE",
        "\r\n:62F:C230209EUR102,00",
        "\r\n-"
    ));

    let lines = result.get_flat_data();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].valuta.as_deref(), Some("230209"));
    assert_eq!(lines[0].bdate.as_deref(), Some("230209"));
    assert_eq!(
        lines[0].saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("230209 102.00 EUR")
    );
}

#[test]
fn kums_mt940_parser_extracts_debit_refs_and_optional_values_like_original() {
    let mut result = GvrKUms::new();
    result.append_mt940_data(concat!(
        "\r\n:20:STARTUMS",
        "\r\n:25:12345678/1234567890",
        "\r\n:60F:C260601EUR100,00",
        "\r\n:61:2606020602D10,50NMSCREF",
        "\r\n/OCMT/USD12,34/CHGS/EUR0,56",
        "\r\n:62F:C260602EUR89,50",
        "\r\n-"
    ));

    let lines = result.get_flat_data();
    assert_eq!(lines.len(), 1);
    let line = lines[0];
    assert_eq!(line.valuta.as_deref(), Some("260602"));
    assert_eq!(line.bdate.as_deref(), Some("260602"));
    assert_eq!(
        line.value.as_ref().map(ToString::to_string).as_deref(),
        Some("-10.50 EUR")
    );
    assert_eq!(
        line.saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("260602 89.50 EUR")
    );
    assert_eq!(line.customerref.as_deref(), Some("REF"));
    assert_eq!(line.instref.as_deref(), Some(""));
    assert_eq!(
        line.orig_value.as_ref().map(ToString::to_string).as_deref(),
        Some("12.34 USD")
    );
    assert_eq!(
        line.charge_value
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("0.56 EUR")
    );
}

#[test]
fn kums_mt940_parser_maps_sepa_counter_account_like_original() {
    let mut result = GvrKUms::new();
    result.append_mt940_data(concat!(
        "\r\n:20:STARTUMS",
        "\r\n:25:12345678/1234567890",
        "\r\n:60F:C260601EUR100,00",
        "\r\n:61:2606020602C10,50NMSCREF",
        "\r\n:86:152?00SEPA CREDIT?20Usage 1?21Usage 2",
        "?30GENODEF1S06 SVWZ+ ja?31DE02123456780000000000",
        "?32Max Mustermann?33Firma GmbH?34EREF?60Add usage",
        "\r\n:62F:C260602EUR110,50",
        "\r\n-"
    ));

    let lines = result.get_flat_data();
    assert_eq!(lines.len(), 1);
    let line = lines[0];
    assert_eq!(line.gvcode.as_deref(), Some("152"));
    assert!(line.is_sepa);
    assert_eq!(line.text.as_deref(), Some("SEPA CREDIT"));
    assert_eq!(line.usage, vec!["Usage 1", "Usage 2", "Add usage"]);
    let other = line.other.as_ref().expect("counter account is present");
    assert_eq!(other.blz.as_deref(), Some("GENODEF1S06"));
    assert_eq!(other.bic.as_deref(), Some("GENODEF1S06"));
    assert_eq!(other.number.as_deref(), Some("DE02123456780000000000"));
    assert_eq!(other.iban.as_deref(), Some("DE02123456780000000000"));
    assert_eq!(other.name.as_deref(), Some("Max Mustermann"));
    assert_eq!(other.name2.as_deref(), Some("Firma GmbH"));
    assert_eq!(line.addkey.as_deref(), Some("EREF"));
}

#[test]
fn kums_mt940_parser_keeps_unknown_999_multitag_as_additional_like_original() {
    let mut result = GvrKUms::new();
    result.append_mt940_data(concat!(
        "\r\n:20:STARTUMS",
        "\r\n:25:12345678/1234567890",
        "\r\n:60F:C260601EUR100,00",
        "\r\n:61:2606020602C10,50NMSCREF",
        "\r\n:86:999RAW\r\n?00NOT STRUCTURED",
        "\r\n:62F:C260602EUR110,50",
        "\r\n-"
    ));

    let lines = result.get_flat_data();
    assert_eq!(lines.len(), 1);
    let line = lines[0];
    assert_eq!(line.gvcode.as_deref(), Some("999"));
    assert_eq!(line.additional.as_deref(), Some("RAW?00NOT STRUCTURED"));
    assert_eq!(line.text, None);
    assert!(line.usage.is_empty());
    assert_eq!(line.other, None);
    assert!(!line.is_sepa);
}

#[test]
fn kums_mt940_parser_extracts_institution_reference_like_original() {
    let mut result = GvrKUms::new();
    result.append_mt940_data(concat!(
        "\r\n:20:STARTUMS",
        "\r\n:25:12345678/1234567890",
        "\r\n:60F:C260601EUR100,00",
        "\r\n:61:2606020602C10,50NMSCREF//INSTREF",
        "\r\n:62F:C260602EUR110,50",
        "\r\n-"
    ));

    let lines = result.get_flat_data();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].customerref.as_deref(), Some("REF"));
    assert_eq!(lines[0].instref.as_deref(), Some("INSTREF"));
    assert_eq!(lines[0].orig_value, None);
    assert_eq!(lines[0].charge_value, None);
}

#[test]
fn kums_mt940_parser_handles_storno_and_year_correction_like_original() {
    let mut result = GvrKUms::new();
    result.append_mt940_data(concat!(
        "\r\n:20:STARTUMS",
        "\r\n:25:12345678/1234567890",
        "\r\n:60F:C231231EUR100,00",
        "\r\n:61:2401021231RC5,00NMSCSTORNO",
        "\r\n:62F:C240102EUR95,00",
        "\r\n-"
    ));

    let lines = result.get_flat_data();
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].valuta.as_deref(), Some("240102"));
    assert_eq!(lines[0].bdate.as_deref(), Some("231231"));
    assert!(lines[0].is_storno);
    assert_eq!(
        lines[0].value.as_ref().map(ToString::to_string).as_deref(),
        Some("-5.00 EUR")
    );
    assert_eq!(
        lines[0].saldo.as_ref().map(ToString::to_string).as_deref(),
        Some("231231 95.00 EUR")
    );
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
