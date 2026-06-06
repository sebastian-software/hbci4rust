use hbci4rust::swift::{Mt940Document, decode_umlauts, get_tag_value};

#[test]
fn decodes_swift_umlaut_placeholders_like_original() {
    assert_eq!(
        decode_umlauts("M[LLER\\BANK]BERWEISUNG~"),
        "MÄLLERÖBANKÜBERWEISUNGß"
    );
}

#[test]
fn decode_umlauts_keeps_unmapped_characters_like_original() {
    assert_eq!(
        decode_umlauts("äöü{}^` already unicode"),
        "äöü{}^` already unicode"
    );
}

#[test]
fn mt940_document_preserves_raw_input_until_parser_is_ported() {
    let document = Mt940Document::parse(":20:START\r\n:86:M[LLER");

    assert_eq!(document.raw, ":20:START\r\n:86:M[LLER");
}

#[test]
fn extracts_mt940_tag_value_like_original() {
    let value = get_tag_value(
        "\r\n:60M:C140106EUR1,00\r\n:61:1401060106CR5,00N062NONREF",
        "60M",
        0,
    );

    assert_eq!(value.as_deref(), Some("C140106EUR1,00"));
}

#[test]
fn extracts_tag_before_broken_dash_inline_tag_like_original() {
    let value = get_tag_value(
        "\r\n:60M:C140106EUR1,00\r\n-:61:1401060106CR5,00N062NONREF",
        "60M",
        0,
    );

    assert_eq!(value.as_deref(), Some("C140106EUR1,00"));
}

#[test]
fn extracts_tag_before_broken_dash_line_tag_like_original() {
    let value = get_tag_value(
        "\r\n:60M:C140106EUR1,00\r\n-\r\n:61:1401060106CR5,00N062NONREF",
        "60M",
        0,
    );

    assert_eq!(value.as_deref(), Some("C140106EUR1,00"));
}

#[test]
fn trims_final_crlf_noise_like_original() {
    assert_eq!(
        get_tag_value("\r\n:62F:C150626EUR91,32\r\n", "62F", 0).as_deref(),
        Some("C150626EUR91,32")
    );
    assert_eq!(
        get_tag_value("\r\n:62F:C150626EUR91,32\r\n-\r\n", "62F", 0).as_deref(),
        Some("C150626EUR91,32")
    );
    assert_eq!(
        get_tag_value("\r\n:62F:C150626EUR91,32\r\n\r\n", "62F", 0).as_deref(),
        Some("C150626EUR91,32")
    );
    assert_eq!(
        get_tag_value("\r\n:62F:C150626EUR91,32\n", "62F", 0).as_deref(),
        Some("C150626EUR91,32")
    );
}

#[test]
fn extracts_tag_after_broken_dash_inline_header_like_original() {
    let value = get_tag_value(
        "\r\n:20:STARTUMSE\r\n-:25:12030000/1019815776\r\n:28C:00000/002\r\n:60M:C181031EUR2776,22\r\n",
        "25",
        0,
    );

    assert_eq!(value.as_deref(), Some("12030000/1019815776"));
}

#[test]
fn extracts_counted_tag_occurrence_like_original() {
    let value = get_tag_value(
        "\r\n:61:first\r\n:86:info\r\n:61:second\r\n:62F:end",
        "61",
        1,
    );

    assert_eq!(value.as_deref(), Some("second"));
}

#[test]
fn missing_tag_value_returns_none_like_java_null() {
    assert_eq!(get_tag_value("\r\n:20:START", "61", 0), None);
}
