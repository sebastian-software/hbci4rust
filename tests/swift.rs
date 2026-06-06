use hbci4rust::swift::{Mt940Document, decode_umlauts};

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
