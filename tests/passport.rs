use hbci4rust::{HbciErrorKind, PinTanPassport, PinTanPassportData, UserSig};

#[test]
fn usersig_encodes_pin_and_tan_like_hbci4java() {
    let encoded = UserSig::encode(Some("12345"), Some("987654")).expect("usersig encodes");

    assert_eq!(encoded, b"\x0512345987654");
}

#[test]
fn usersig_encodes_java_nulls_as_empty_strings() {
    let encoded = UserSig::encode(None, None).expect("empty usersig encodes");
    let decoded = UserSig::decode(Some(&encoded)).expect("empty usersig decodes");

    assert_eq!(encoded, [0]);
    assert_eq!(decoded.pin(), "");
    assert_eq!(decoded.tan(), "");
}

#[test]
fn usersig_decodes_pin_and_tan_like_hbci4java() {
    let decoded = UserSig::decode(Some(b"\x0512345987654")).expect("usersig decodes");

    assert_eq!(decoded.pin(), "12345");
    assert_eq!(decoded.tan(), "987654");
}

#[test]
fn usersig_decodes_latin1_bytes_like_fints_wire_encoding() {
    let decoded = UserSig::decode(Some(b"\x04M\xfcll4711")).expect("latin1 usersig decodes");

    assert_eq!(decoded.pin(), format!("M{}ll", char::from(0xfc)));
    assert_eq!(decoded.tan(), "4711");
    assert_eq!(
        decoded.to_bytes().expect("latin1 usersig re-encodes"),
        b"\x04M\xfcll4711"
    );
}

#[test]
fn usersig_rejects_missing_or_invalid_signature_bytes() {
    let missing = UserSig::decode(None).expect_err("missing usersig is rejected");
    assert_eq!(missing.kind(), HbciErrorKind::InvalidArgument);

    let too_short = UserSig::decode(Some(b"\x0512")).expect_err("short usersig is rejected");
    assert_eq!(too_short.kind(), HbciErrorKind::InvalidArgument);
}

#[test]
fn usersig_rejects_non_latin1_text() {
    let err =
        UserSig::encode(Some("1234"), Some("\u{1F510}")).expect_err("non-latin1 TAN is rejected");

    assert_eq!(err.kind(), HbciErrorKind::Unsupported);
}

#[test]
fn pintan_passport_caches_and_clears_runtime_pin() {
    let mut passport = PinTanPassport::new(PinTanPassportData::default());

    assert_eq!(passport.pin(), None);

    passport.set_pin("12345");
    assert_eq!(passport.pin(), Some("12345"));

    passport.clear_pin();
    assert_eq!(passport.pin(), None);
}
