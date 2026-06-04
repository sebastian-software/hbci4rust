use hbci4rust::protocol::parse_wire_message;

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

fn assert_components(field: &hbci4rust::protocol::WireField, expected: &[&str]) {
    let actual = field
        .components()
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}
