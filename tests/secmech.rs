use hbci4rust::{HhdVersion, HhdVersionType, MatrixCode, Properties, QrCode};

#[test]
fn qr_code_extracts_embedded_png_and_message_like_original_test001() {
    let data = latin1_string(include_bytes!(
        "fixtures/hbci4java/secmech/TestQRCode-001.txt"
    ));
    let code = QrCode::new(Some("1234"), Some(&data)).expect("QR code");

    assert_eq!(code.mimetype(), Some("image/png"));
    assert_eq!(code.image().len(), 456);
    assert_eq!(&code.image()[0..4], &[0x89, b'P', b'N', b'G']);
    assert_eq!(code.message(), Some(expected_qr_message().as_str()));
}

#[test]
fn qr_code_rejects_null_like_original_test002() {
    assert!(QrCode::new(None, None).is_err());
    assert!(QrCode::try_parse(None, None).is_none());
}

#[test]
fn matrix_code_extracts_first_upstream_fixture_like_original_test001() {
    let code = MatrixCode::new(Some(include_bytes!(
        "fixtures/hbci4java/secmech/TestMatrixCode-001.txt"
    )))
    .expect("matrix code");

    assert_eq!(code.mimetype(), Some("image/png"));
    assert_eq!(code.image().len(), 4556);
}

#[test]
fn matrix_code_extracts_second_upstream_fixture_like_original_test002() {
    let code = MatrixCode::new(Some(include_bytes!(
        "fixtures/hbci4java/secmech/TestMatrixCode-002.txt"
    )))
    .expect("matrix code");

    assert_eq!(code.mimetype(), Some("image/png"));
    assert_eq!(code.image().len(), 4980);
}

#[test]
fn matrix_code_rejects_null_and_short_text_like_original_tests003_and004() {
    assert!(MatrixCode::new(None).is_err());
    assert!(MatrixCode::from_text(Some("zu kurz")).is_err());
    assert!(MatrixCode::try_parse(Some("zu kurz")).is_none());
}

#[test]
fn hhd_version_detects_matrix_code_from_secmech_like_original_tests005_and006() {
    assert_eq!(
        HhdVersion::find(Some(&properties(&[("id", "MS1.0.0"), ("segversion", "5")]))),
        HhdVersion::Ms1
    );
    assert_eq!(
        HhdVersion::find(Some(&properties(&[
            ("id", "photoTAN"),
            ("name", "photoTAN QRcode"),
            ("segversion", "6")
        ]))),
        HhdVersion::Ms1
    );
    assert_eq!(HhdVersion::Ms1.hhd_type(), HhdVersionType::PhotoTan);
}

#[test]
fn hhd_version_matches_upstream_test_hhd_version_cases() {
    let cases = [
        (
            HhdVersion::Ms1,
            &[("id", "MS1.0.0"), ("segversion", "5")][..],
        ),
        (
            HhdVersion::Hhd14,
            &[
                ("id", "CR#5 - 1.4"),
                ("segversion", "5"),
                ("zkamethod_version", "1.4"),
            ][..],
        ),
        (
            HhdVersion::Hhd14,
            &[
                ("id", "CR#6 - 1.4"),
                ("segversion", "5"),
                ("zkamethod_version", "1.4"),
            ][..],
        ),
        (
            HhdVersion::Hhd13,
            &[("id", "HHD1.3.0"), ("segversion", "3")][..],
        ),
        (
            HhdVersion::Hhd13,
            &[("id", "HHD1.3.0OPT"), ("segversion", "3")][..],
        ),
        (
            HhdVersion::Hhd13,
            &[("id", "HHD1.3.0USB"), ("segversion", "3")][..],
        ),
        (
            HhdVersion::Qr13,
            &[("id", "HHD1.3.0QR"), ("segversion", "3")][..],
        ),
        (HhdVersion::Qr14, &[("id", "Q1S"), ("segversion", "5")][..]),
        (
            HhdVersion::Hhd13,
            &[
                ("id", "mTAN"),
                ("name", "smsTAN"),
                ("zkamethod_version", "1.3.2"),
                ("segversion", "4"),
            ][..],
        ),
        (
            HhdVersion::Hhd13,
            &[("id", "mTAN"), ("zkamethod_version", "1.3")][..],
        ),
        (
            HhdVersion::Decoupled,
            &[("zkamethod_name", "Decoupled")][..],
        ),
        (
            HhdVersion::Decoupled,
            &[("zkamethod_name", "DecoupledPush")][..],
        ),
    ];

    for (expected, props) in cases {
        assert_eq!(HhdVersion::find(Some(&properties(props))), expected);
    }

    assert_eq!(HhdVersion::find(None), HhdVersion::DEFAULT);
    assert_eq!(HhdVersion::Qr13.challenge_version(), Some("hhd13"));
    assert_eq!(HhdVersion::Decoupled.challenge_version(), None);
}

fn properties(values: &[(&str, &str)]) -> Properties {
    values
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn latin1_string(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}

fn expected_qr_message() -> String {
    latin1_string(
        b"Sie haben eine \"Einzel\xfcberweisung\" erfasst: \xdcberpr\xfcfen Sie die Richtigkeit der \"letzten 10 Zeichen der IBAN des Empf\xe4ngers\" bei dem Institut \"MUSTER-BANK\" und best\xe4tigen Sie diese mit der Taste \"OK\". \xdcberpr\xfcfen Sie die Richtigkeit des \"Betrags\" und best\xe4tigen Sie diesen mit der Taste \"OK\".",
    )
}
