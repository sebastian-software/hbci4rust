use hbci4rust::{
    ChallengeHhdVersion, ChallengeInfo, FlickerCode, FlickerCodeVersion, FlickerRenderer,
    HhdVersion, HhdVersionType, MatrixCode, Properties, QrCode,
};

const CHALLENGE_DATA: &str = include_str!("fixtures/hbci4java/secmech/challengedata.xml");

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

#[test]
fn challenge_info_unknown_job_returns_none_like_original_test_invalid() {
    let info = challenge_info();
    assert!(
        info.get_data("UNDEF")
            .and_then(|job| job.hhd_version(HhdVersion::Hhd14))
            .is_none()
    );
}

#[test]
fn challenge_info_missing_params_matches_original_test_missing() {
    let info = challenge_info();
    let version = challenge_version(&info, "HKDTE", HhdVersion::Hhd14);
    assert_eq!(version.params().len(), 0);
}

#[test]
fn challenge_info_classes_match_original_test_klass() {
    let info = challenge_info();

    assert_eq!(
        challenge_version(&info, "HKAOM", HhdVersion::Hhd12).klass(),
        "20"
    );
    assert_eq!(
        challenge_version(&info, "HKAOM", HhdVersion::Hhd13).klass(),
        "20"
    );
    assert_eq!(
        challenge_version(&info, "HKAOM", HhdVersion::Hhd14).klass(),
        "10"
    );

    assert_eq!(
        challenge_version(&info, "HKCCS", HhdVersion::Hhd12).klass(),
        "22"
    );
    assert_eq!(
        challenge_version(&info, "HKCCS", HhdVersion::Hhd13).klass(),
        "22"
    );
    assert_eq!(
        challenge_version(&info, "HKCCS", HhdVersion::Hhd14).klass(),
        "09"
    );
}

#[test]
fn challenge_info_formats_wrt_like_original_test_wrt() {
    let info = challenge_info();

    assert_wrt_param(
        &challenge_version(&info, "HKAOM", HhdVersion::Hhd12).params()[1],
        "BTG.value",
    );
    assert_wrt_param(
        &challenge_version(&info, "HKAOM", HhdVersion::Hhd13).params()[2],
        "BTG.value",
    );
    assert_wrt_param(
        &challenge_version(&info, "HKAOM", HhdVersion::Hhd14).params()[0],
        "BTG.value",
    );
}

#[test]
fn challenge_info_formats_blank_type_without_escaping_like_original_test_an() {
    let info = challenge_info();

    assert_blank_type_param(
        &challenge_version(&info, "HKAOM", HhdVersion::Hhd12).params()[0],
        "Other.number",
    );
    assert_blank_type_param(
        &challenge_version(&info, "HKAOM", HhdVersion::Hhd13).params()[1],
        "Other.number",
    );
    assert_blank_type_param(
        &challenge_version(&info, "HKAOM", HhdVersion::Hhd14).params()[3],
        "Other.number",
    );
}

#[test]
fn challenge_info_formats_date_like_original_test_date() {
    let info = challenge_info();
    let param = &challenge_version(&info, "HKTUE", HhdVersion::Hhd14).params()[3];

    assert_eq!(param.path(), Some("date"));
    assert_eq!(param.param_type(), "Date");
    assert_eq!(
        param.format(Some("2011-05-20")).expect("date"),
        Some("20110520".to_owned())
    );
    assert_eq!(param.format(None).expect("none"), None);
    assert!(param.format(Some("invalid-date")).is_err());
}

#[test]
fn challenge_info_conditions_match_original_tests_condition_and_condition2() {
    let info = challenge_info();
    let no_challenge_value = properties(&[("needchallengevalue", "N")]);
    let need_challenge_value = properties(&[("needchallengevalue", "J")]);

    for version in [HhdVersion::Hhd12, HhdVersion::Hhd13] {
        for param in challenge_version(&info, "HKAOM", version).params() {
            if param.path() == Some("BTG.value") {
                assert!(!param.is_complied(&no_challenge_value));
            }
        }
    }
    for param in challenge_version(&info, "HKAOM", HhdVersion::Hhd14).params() {
        if param.path() == Some("BTG.value") {
            assert!(param.is_complied(&no_challenge_value));
        }
    }

    for version in [HhdVersion::Hhd12, HhdVersion::Hhd13, HhdVersion::Hhd14] {
        for param in challenge_version(&info, "HKCCS", version).params() {
            if param.path() == Some("sepa.btg.value") {
                assert!(param.is_complied(&need_challenge_value));
            }
        }
    }
}

fn properties(values: &[(&str, &str)]) -> Properties {
    values
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect()
}

fn challenge_info() -> ChallengeInfo {
    ChallengeInfo::parse_xml(CHALLENGE_DATA).expect("challenge info")
}

fn challenge_version<'a>(
    info: &'a ChallengeInfo,
    code: &str,
    version: HhdVersion,
) -> &'a ChallengeHhdVersion {
    info.get_data(code)
        .and_then(|job| job.hhd_version(version))
        .expect("challenge version")
}

fn assert_wrt_param(param: &hbci4rust::ChallengeParam, path: &str) {
    assert_eq!(param.path(), Some(path));
    assert_eq!(param.param_type(), "Wrt");
    assert_eq!(
        param.format(Some("100")).expect("wrt"),
        Some("100,".to_owned())
    );
    assert_eq!(
        param.format(Some("100.50")).expect("wrt"),
        Some("100,5".to_owned())
    );
    assert_eq!(
        param.format(Some("100.99")).expect("wrt"),
        Some("100,99".to_owned())
    );
    assert_eq!(param.format(None).expect("none"), None);
}

fn assert_blank_type_param(param: &hbci4rust::ChallengeParam, path: &str) {
    assert_eq!(param.path(), Some(path));
    assert_eq!(param.param_type(), "");
    assert_eq!(
        param.format(Some("AaBb")).expect("an"),
        Some("AaBb".to_owned())
    );
    assert_eq!(
        param.format(Some("+:'@")).expect("an"),
        Some("+:'@".to_owned())
    );
    assert_eq!(param.format(None).expect("none"), None);
}

fn latin1_string(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| char::from(*byte)).collect()
}

fn expected_qr_message() -> String {
    latin1_string(
        b"Sie haben eine \"Einzel\xfcberweisung\" erfasst: \xdcberpr\xfcfen Sie die Richtigkeit der \"letzten 10 Zeichen der IBAN des Empf\xe4ngers\" bei dem Institut \"MUSTER-BANK\" und best\xe4tigen Sie diese mit der Taste \"OK\". \xdcberpr\xfcfen Sie die Richtigkeit des \"Betrags\" und best\xe4tigen Sie diesen mit der Taste \"OK\".",
    )
}

#[test]
fn flicker_code_parses_hhd14_user_fixture_like_original_test1() {
    let code = FlickerCode::new("039870110490631098765432100812345678041,00").expect("flicker");

    assert_eq!(code.lc, 39);
    assert_eq!(code.version, Some(FlickerCodeVersion::Hhd14));
    assert_eq!(code.start_code.element.lde, 135);
    assert_eq!(code.start_code.element.length, 7);
    assert_eq!(code.start_code.element.data.as_deref(), Some("1049063"));
    assert_eq!(code.start_code.control_bytes, vec![1]);
    assert_eq!(code.de1.data.as_deref(), Some("9876543210"));
    assert_eq!(code.de2.data.as_deref(), Some("12345678"));
    assert_eq!(code.de3.data.as_deref(), Some("1,00"));
    assert_eq!(
        code.render().expect("render"),
        "1784011049063F059876543210041234567844312C303019"
    );
}

#[test]
fn flicker_code_renders_upstream_test2_to_test5_cases() {
    assert_flicker_render(
        "039870110418751012345678900812030000040,20",
        "1784011041875F051234567890041203000044302C323015",
    );
    assert_flicker_render(
        "0248A0120452019980812345678",
        "0D85012045201998041234567855",
    );
    assert_flicker_render(
        "...TAN-Nummer: CHLGUC 002624088715131306389726041,00CHLGTEXT0244 Sie h...",
        "0F04871513130338972614312C30303B",
    );
    assert_flicker_render(
        "0248A01204520199808123F5678",
        "118501204520199848313233463536373875",
    );
}

#[test]
fn flicker_code_manual_luhn_zero_matches_original_test6() {
    let mut code = FlickerCode {
        version: Some(FlickerCodeVersion::Hhd14),
        ..FlickerCode::default()
    };
    code.start_code.element.data = Some("1120492".to_owned());
    code.start_code.control_bytes.push(1);
    code.de1.data = Some("30084403".to_owned());
    code.de2.data = Some("450,00".to_owned());
    code.de3.data = Some("2".to_owned());

    assert_eq!(
        code.render().expect("render"),
        "1584011120492F0430084403463435302C3030012F05"
    );
}

#[test]
fn flicker_code_parses_hhd13_and_fallback_like_original_tests7_and8() {
    let code = FlickerCode::new("190277071234567041,00").expect("flicker");
    assert_eq!(code.version, Some(FlickerCodeVersion::Hhd13));
    assert_eq!(code.lc, 19);
    assert_eq!(code.start_code.element.lde, 2);
    assert_eq!(code.start_code.element.length, 2);
    assert_eq!(code.start_code.element.data.as_deref(), Some("77"));
    assert_eq!(code.de1.data.as_deref(), Some("1234567"));
    assert_eq!(code.de2.data.as_deref(), Some("1,00"));

    let fallback = FlickerCode::new("250891715637071234567041,00").expect("flicker");
    assert_eq!(fallback.version, Some(FlickerCodeVersion::Hhd13));
    assert!(fallback.start_code.control_bytes.is_empty());
}

#[test]
fn flicker_code_sparda_three_digit_lde_matches_original_test9() {
    let code =
        FlickerCode::new("044880120932160022DE125001051706484898900041,00").expect("flicker");

    assert_eq!(code.version, Some(FlickerCodeVersion::Hhd14));
    assert_eq!(code.de1.lde_len, 3);
    assert_eq!(code.de1.length, 22);
    assert_eq!(code.de1.data.as_deref(), Some("DE12500105170648489890"));
    assert_eq!(
        code.render().expect("render"),
        "23840120932160564445313235303031303531373036343834383938393044312C303005"
    );
}

#[test]
fn flicker_code_try_parse_with_hhd_version_matches_original_test10() {
    let hhd = HhdVersion::find(Some(&properties(&[
        ("id", "HHD1.3.2OPT"),
        ("name", "chipTAN optisch"),
        ("secfunc", "911"),
        ("zkamethod_name", "HHDOPT1"),
        ("zkamethod_version", "1.3.2"),
    ])));
    let code = FlickerCode::try_parse(Some(hhd), None, Some("2908881904551000000039990515,00"))
        .expect("flicker");

    assert_eq!(
        code.render().expect("render"),
        "1204881904550500000039991531352C30308A"
    );
}

#[test]
fn flicker_renderer_generates_original_two_iteration_frame_sequence() {
    let code = FlickerCode::new("0248A0120452019980812345678").expect("flicker");
    let flicker = code.render().expect("render");
    let renderer = FlickerRenderer::new(&flicker).expect("renderer");

    assert_eq!(
        render_frame_string(&renderer.frames_for_iterations(2)),
        expected_flicker_frames()
    );
}

fn assert_flicker_render(input: &str, expected: &str) {
    let code = FlickerCode::new(input).expect("flicker");
    assert_eq!(code.render().expect("render"), expected);
}

fn render_frame_string(frames: &[[bool; 5]]) -> String {
    let mut rendered = String::new();
    for frame in frames {
        for bit in frame {
            rendered.push(if *bit { '1' } else { '0' });
        }
        rendered.push(' ');
    }
    rendered
}

fn expected_flicker_frames() -> String {
    let once = "11111 01111 10000 00000 11111 01111 11111 01111 11011 01011 10000 00000 11010 01010 10001 00001 11000 01000 10000 00000 10000 00000 10100 00100 11010 01010 10010 00010 10000 00000 10100 00100 11001 01001 11000 01000 10001 00001 11001 01001 10010 00010 10000 00000 10100 00100 11000 01000 10010 00010 11100 01100 10110 00110 11010 01010 10001 00001 11110 01110 11010 01010 11010 01010 ";
    format!("{once}{once}")
}
