use hbci4rust::swift::{
    Mt940Document, decode_umlauts, get_multi_tag_value, get_one_block, get_tag_value, pack_multi,
};

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
fn one_block_returns_none_for_empty_stream_like_java_null() {
    assert_eq!(get_one_block(""), None);
}

#[test]
fn one_block_returns_whole_single_block_like_original() {
    let stream = "\r\n:20:FIRST\r\n:25:12030000/1019815776\r\n:62F:C150626EUR91,32\r\n-";

    assert_eq!(get_one_block(stream).as_deref(), Some(stream));
}

#[test]
fn one_block_splits_before_next_20_marker_like_original() {
    let first = "\r\n:20:FIRST\r\n:25:12030000/1019815776\r\n-";
    let second = "\r\n:20:SECOND\r\n:25:12030000/1019815777\r\n-";
    let stream = format!("{first}{second}");

    assert_eq!(get_one_block(&stream).as_deref(), Some(first));
}

#[test]
fn one_block_search_starts_after_first_byte_like_original() {
    let stream = "\r\n:20:FIRST\r\n:25:A";

    assert_eq!(get_one_block(stream).as_deref(), Some(stream));
}

#[test]
fn one_block_returns_prefix_before_first_later_marker_like_original() {
    let stream = "REST\r\n:20:FIRST\r\n:25:A";

    assert_eq!(get_one_block(stream).as_deref(), Some("REST"));
}

#[test]
fn pack_multi_removes_crlf_pairs_like_original() {
    assert_eq!(
        pack_multi("?00Text\r\n?20Usage\r?30Other\n"),
        "?00Text?20Usage\r?30Other\n"
    );
}

#[test]
fn extracts_multi_tag_value_until_next_numeric_code_like_original() {
    let value = get_multi_tag_value("?00BOOKING TEXT?10PN123?20Usage", "00");

    assert_eq!(value.as_deref(), Some("BOOKING TEXT"));
}

#[test]
fn multi_tag_value_keeps_question_marks_without_two_digits_like_original() {
    let value = get_multi_tag_value("?20hello?xworld??still text?30next", "20");

    assert_eq!(value.as_deref(), Some("hello?xworld??still text"));
}

#[test]
fn multi_tag_value_uses_tail_when_question_marker_is_too_short_like_original() {
    let value = get_multi_tag_value("?20hello?3", "20");

    assert_eq!(value.as_deref(), Some("hello?3"));
}

#[test]
fn multi_tag_value_returns_none_for_missing_tag_like_java_null() {
    assert_eq!(get_multi_tag_value("?20hello", "30"), None);
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
