use hbci4rust::HbciErrorKind;
use hbci4rust::protocol::{load_protocol_spec, parse_wire_message};

#[test]
fn parses_segments_fields_and_components() {
    let message = parse_wire_message(
        "HNHBK:1:3+000000000123+300+DIALOG1+1'HIRMG:2:2+0010::Nachricht erhalten'",
    )
    .expect("wire message parses");

    assert_eq!(message.len(), 2);
    assert_eq!(message.segments()[0].code(), Some("HNHBK"));
    assert_eq!(message.segments()[0].sequence(), Some("1"));
    assert_eq!(message.segments()[0].version(), Some("3"));
    assert_eq!(
        message.segments()[0].fields()[1].value(),
        Some("000000000123")
    );
    assert_eq!(message.segments()[0].fields()[3].value(), Some("DIALOG1"));

    assert_eq!(message.segments()[1].code(), Some("HIRMG"));
    assert_components(
        &message.segments()[1].fields()[1],
        &["0010", "", "Nachricht erhalten"],
    );
}

#[test]
fn parses_quoted_delimiters_and_binary_blocks() {
    let message = parse_wire_message("HITAN:3:6+Text?+plus?:colon??question?@at+@5@A+B:C+tail'")
        .expect("wire message parses");

    let segment = &message.segments()[0];
    assert_eq!(
        segment.fields()[1].value(),
        Some("Text+plus:colon?question@at")
    );
    assert_eq!(segment.fields()[2].value(), Some("@5@A+B:C"));
    assert_eq!(segment.fields()[3].value(), Some("tail"));
}

#[test]
fn preserves_empty_fields_and_components() {
    let message = parse_wire_message("HITAN:3:6++alpha::gamma+'").expect("wire message parses");

    let segment = &message.segments()[0];
    assert_eq!(segment.fields()[1].value(), Some(""));
    assert_components(&segment.fields()[2], &["alpha", "", "gamma"]);
    assert_eq!(segment.fields()[3].value(), Some(""));
}

#[test]
fn rejects_malformed_wire_messages() {
    assert!(parse_wire_message("HIRMG:1:2+foo").is_err());
    assert!(parse_wire_message("HIRMG:1:2+foo?'").is_err());
    assert!(parse_wire_message("HNVSD:1:1+@5@ab'").is_err());
    assert!(parse_wire_message("HNVSD:1:1+@x@ab'").is_err());
}

#[test]
fn resolves_wire_segments_to_protocol_definitions() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");
    let message = parse_wire_message("HNHBK:1:3+000000000123+300+DIALOG1+1'HIRMG:2:2+0010::ok'")
        .expect("wire message parses");

    let resolved = message
        .resolve_segments(&syntax)
        .expect("wire segments resolve");

    assert_eq!(resolved.len(), 2);
    assert_eq!(resolved.segments()[0].definition().id, "MsgHeadInst");
    assert_eq!(resolved.segments()[0].code(), Some("HNHBK"));
    assert_eq!(resolved.segments()[0].sequence(), Some("1"));
    assert_eq!(resolved.segments()[0].version(), Some("3"));
    assert_eq!(resolved.segments()[1].definition().id, "RetGlob");
}

#[test]
fn extracts_flat_values_from_resolved_message_head_segment() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");
    let message = parse_wire_message("HNHBK:1:3+000000000123+300+DIALOG1+1+DIALOG0:1'")
        .expect("wire message parses");
    let resolved = message
        .resolve_segments(&syntax)
        .expect("wire segments resolve");

    let values = resolved.segments()[0]
        .values(&syntax)
        .expect("segment values extract");

    assert_eq!(
        values.get("MsgHeadInst.SegHead.code").map(String::as_str),
        Some("HNHBK")
    );
    assert_eq!(
        values.get("MsgHeadInst.SegHead.seq").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        values
            .get("MsgHeadInst.SegHead.version")
            .map(String::as_str),
        Some("3")
    );
    assert_eq!(
        values.get("MsgHeadInst.msgsize").map(String::as_str),
        Some("000000000123")
    );
    assert_eq!(
        values.get("MsgHeadInst.dialogid").map(String::as_str),
        Some("DIALOG1")
    );
    assert_eq!(
        values.get("MsgHeadInst.msgnum").map(String::as_str),
        Some("1")
    );
    assert_eq!(
        values
            .get("MsgHeadInst.MsgRef.dialogid")
            .map(String::as_str),
        Some("DIALOG0")
    );
    assert_eq!(
        values.get("MsgHeadInst.MsgRef.msgnum").map(String::as_str),
        Some("1")
    );
}

#[test]
fn extracts_flat_values_from_repeated_data_element_groups() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");
    let message = parse_wire_message(
        "HIRMG:2:2+0010::Nachricht erhalten+0020:ABC:Zweite Meldung:param1:param2'",
    )
    .expect("wire message parses");
    let resolved = message
        .resolve_segments(&syntax)
        .expect("wire segments resolve");

    let values = resolved.segments()[0]
        .values(&syntax)
        .expect("segment values extract");

    assert_eq!(
        values.get("RetGlob.SegHead.code").map(String::as_str),
        Some("HIRMG")
    );
    assert_eq!(
        values.get("RetGlob.SegHead.seq").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        values.get("RetGlob.SegHead.version").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        values.get("RetGlob.RetVal.code").map(String::as_str),
        Some("0010")
    );
    assert_eq!(
        values.get("RetGlob.RetVal.ref").map(String::as_str),
        Some("")
    );
    assert_eq!(
        values.get("RetGlob.RetVal.text").map(String::as_str),
        Some("Nachricht erhalten")
    );
    assert_eq!(
        values.get("RetGlob.RetVal_2.code").map(String::as_str),
        Some("0020")
    );
    assert_eq!(
        values.get("RetGlob.RetVal_2.ref").map(String::as_str),
        Some("ABC")
    );
    assert_eq!(
        values.get("RetGlob.RetVal_2.text").map(String::as_str),
        Some("Zweite Meldung")
    );
    assert_eq!(
        values.get("RetGlob.RetVal_2.parm").map(String::as_str),
        Some("param1")
    );
    assert_eq!(
        values.get("RetGlob.RetVal_2.parm_2").map(String::as_str),
        Some("param2")
    );
}

#[test]
fn extracts_datatype_parsed_values_from_resolved_segments() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");
    let message =
        parse_wire_message("HIPRO:3:4+DIALOG1:1+2+20240229+070809+0010::Status erhalten'")
            .expect("wire message parses");
    let resolved = message
        .resolve_segments(&syntax)
        .expect("wire segments resolve");

    let values = resolved.segments()[0]
        .values(&syntax)
        .expect("segment values extract");

    assert_eq!(resolved.segments()[0].definition().id, "StatusRes4");
    assert_eq!(
        values.get("StatusRes4.date").map(String::as_str),
        Some("2024-02-29")
    );
    assert_eq!(
        values.get("StatusRes4.time").map(String::as_str),
        Some("07:08:09")
    );
    assert_eq!(
        values.get("StatusRes4.segref").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        values.get("StatusRes4.RetVal.text").map(String::as_str),
        Some("Status erhalten")
    );
}

#[test]
fn extracts_flat_values_from_resolved_wire_messages() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");
    let message = parse_wire_message(
        "HNHBK:1:3+000000000123+300+DIALOG1+1+DIALOG0:1'HIRMG:2:2+0010::Nachricht erhalten'",
    )
    .expect("wire message parses");
    let resolved = message
        .resolve_segments(&syntax)
        .expect("wire segments resolve");

    let values = resolved.values(&syntax).expect("message values extract");

    assert_eq!(
        values.get("MsgHeadInst.SegHead.code").map(String::as_str),
        Some("HNHBK")
    );
    assert_eq!(
        values
            .get("MsgHeadInst.MsgRef.dialogid")
            .map(String::as_str),
        Some("DIALOG0")
    );
    assert_eq!(
        values.get("RetGlob.RetVal.text").map(String::as_str),
        Some("Nachricht erhalten")
    );
}

#[test]
fn suffixes_repeated_segment_roots_in_resolved_wire_message_values() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");
    let message =
        parse_wire_message("HIRMG:1:2+0010::Erste Meldung'HIRMG:2:2+0020::Zweite Meldung'")
            .expect("wire message parses");
    let resolved = message
        .resolve_segments(&syntax)
        .expect("wire segments resolve");

    let values = resolved.values(&syntax).expect("message values extract");

    assert_eq!(
        values.get("RetGlob.RetVal.text").map(String::as_str),
        Some("Erste Meldung")
    );
    assert_eq!(
        values.get("RetGlob_2.RetVal.text").map(String::as_str),
        Some("Zweite Meldung")
    );
    assert_eq!(
        values.get("RetGlob_2.SegHead.seq").map(String::as_str),
        Some("2")
    );
}

#[test]
fn extracts_values_for_direct_segment_message_definitions() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");
    let message = parse_wire_message(
        "HNHBK:1:3+000000000123+300+DIALOG1+1+DIALOG0:1'HIRMG:2:2+0010::Dialog beendet'HNHBS:3:1+1'",
    )
    .expect("wire message parses");
    let resolved = message
        .resolve_segments(&syntax)
        .expect("wire segments resolve");

    let values = resolved
        .values_for_message(&syntax, "DialogEndRes")
        .expect("message values extract");

    assert_eq!(
        values
            .get("DialogEndRes.MsgHead.SegHead.code")
            .map(String::as_str),
        Some("HNHBK")
    );
    assert_eq!(
        values
            .get("DialogEndRes.MsgHead.MsgRef.dialogid")
            .map(String::as_str),
        Some("DIALOG0")
    );
    assert_eq!(
        values
            .get("DialogEndRes.RetGlob.RetVal.text")
            .map(String::as_str),
        Some("Dialog beendet")
    );
    assert_eq!(
        values
            .get("DialogEndRes.MsgTail.msgnum")
            .map(String::as_str),
        Some("1")
    );
}

#[test]
fn rejects_wire_messages_that_do_not_match_the_message_definition() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");
    let message = parse_wire_message(
        "HNHBK:1:3+000000000123+300+DIALOG1+1+DIALOG0:1'HIRMG:2:2+0010::Dialog beendet'HNHBS:3:1+1'",
    )
    .expect("wire message parses");
    let resolved = message
        .resolve_segments(&syntax)
        .expect("wire segments resolve");

    let err = resolved
        .values_for_message(&syntax, "DialogEnd")
        .expect_err("wrong message definition is rejected");
    assert_eq!(err.kind(), HbciErrorKind::Protocol);
}

#[test]
fn skips_absent_optional_syntax_functions_in_message_mapping() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");
    let message = parse_wire_message(
        "HNHBK:1:3+000000000123+300+DIALOG1+1+DIALOG0:1'HIRMG:2:2+0010::Initialisiert'HNHBS:3:1+1'",
    )
    .expect("wire message parses");
    let resolved = message
        .resolve_segments(&syntax)
        .expect("wire segments resolve");

    let values = resolved
        .values_for_message(&syntax, "DialogInitRes")
        .expect("message values extract");

    assert_eq!(
        values
            .get("DialogInitRes.MsgHead.SegHead.code")
            .map(String::as_str),
        Some("HNHBK")
    );
    assert_eq!(
        values
            .get("DialogInitRes.RetGlob.RetVal.text")
            .map(String::as_str),
        Some("Initialisiert")
    );
    assert_eq!(
        values
            .get("DialogInitRes.MsgTail.msgnum")
            .map(String::as_str),
        Some("1")
    );
}

#[test]
fn validates_resolved_segment_sequence_numbers() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");
    let message = parse_wire_message(
        "HNHBK:1:3+000000000123+300+DIALOG1+1'HIRMG:2:2+0010::Nachricht erhalten'",
    )
    .expect("wire message parses");
    let resolved = message
        .resolve_segments(&syntax)
        .expect("wire segments resolve");

    resolved
        .validate_segment_sequence()
        .expect("segment sequence validates");
    assert_eq!(
        resolved
            .check_segment_sequence(1)
            .expect("segment sequence validates"),
        3
    );
}

#[test]
fn rejects_wrong_or_invalid_segment_sequence_numbers() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let skipped_message =
        parse_wire_message("HNHBK:1:3+000000000123+300+DIALOG1+1'HIRMG:3:2+0010::ok'")
            .expect("wire message parses");
    let skipped = skipped_message
        .resolve_segments(&syntax)
        .expect("wire segments resolve");
    let skipped_err = skipped
        .validate_segment_sequence()
        .expect_err("skipped sequence is rejected");
    assert_eq!(skipped_err.kind(), HbciErrorKind::Protocol);

    let non_numeric_message =
        parse_wire_message("HNHBK:1:3+000000000123+300+DIALOG1+1'HIRMG:x:2+0010::ok'")
            .expect("wire message parses");
    let non_numeric = non_numeric_message
        .resolve_segments(&syntax)
        .expect("wire segments resolve");
    let non_numeric_err = non_numeric
        .validate_segment_sequence()
        .expect_err("non-numeric sequence is rejected");
    assert_eq!(non_numeric_err.kind(), HbciErrorKind::Protocol);
}

#[test]
fn rejects_unknown_or_incomplete_segment_headers_during_resolution() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let unknown = parse_wire_message("ZZZZZ:1:1+foo'").expect("wire message parses");
    let unknown_err = unknown
        .resolve_segments(&syntax)
        .expect_err("unknown segment is rejected");
    assert_eq!(unknown_err.kind(), HbciErrorKind::Protocol);

    let incomplete = parse_wire_message("HIRMG:1+foo'").expect("wire message parses");
    let incomplete_err = incomplete
        .resolve_segments(&syntax)
        .expect_err("missing version is rejected");
    assert_eq!(incomplete_err.kind(), HbciErrorKind::Protocol);
}

fn assert_components(field: &hbci4rust::protocol::WireField, expected: &[&str]) {
    let actual = field
        .components()
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}
