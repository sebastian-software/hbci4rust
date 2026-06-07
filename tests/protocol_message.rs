use hbci4rust::{
    UserSig, apply_pintan_user_sig_to_sig_tail,
    protocol::{HbciMessage, SyntaxElementKind, load_protocol_spec},
};

#[test]
fn builds_message_tree_with_original_paths_and_defaults() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let message = HbciMessage::from_syntax(&syntax, "DialogInit").expect("message tree builds");

    assert_eq!(message.name(), "DialogInit");
    assert_eq!(message.path(), "DialogInit");

    let msg_head = message
        .element("DialogInit.MsgHead")
        .expect("message head exists");
    assert_eq!(msg_head.kind(), SyntaxElementKind::Seg);
    assert_eq!(msg_head.type_name(), "MsgHeadUser");

    assert_eq!(
        message.value("DialogInit.MsgHead.SegHead.code"),
        Some("HNHBK")
    );
    assert_eq!(
        message.value("DialogInit.MsgHead.SegHead.version"),
        Some("3")
    );
    assert_eq!(message.value("DialogInit.MsgHead.hbciversion"), Some("300"));
    assert_eq!(message.value("DialogInit.SigHead.range"), Some("1"));
    assert_eq!(
        message.value("DialogInit.MsgTail.SegHead.code"),
        Some("HNHBS")
    );

    let tan_segment = message
        .element("DialogInit.TAN2Step6")
        .expect("optional TAN2Step6 segment is still built");
    assert_eq!(tan_segment.min_num(), 0);
    assert_eq!(tan_segment.max_num(), 1);

    let msg_size = message
        .element("DialogInit.MsgHead.msgsize")
        .expect("message size data element exists");
    assert_eq!(msg_size.min_size(), Some(12));
    assert_eq!(msg_size.max_size(), Some(12));
}

#[test]
fn builds_custom_message_response_despite_unresolved_valid_metadata() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let message = HbciMessage::from_syntax(&syntax, "CustomMsgRes").expect("message tree builds");

    assert_eq!(message.name(), "CustomMsgRes");
    assert!(
        message
            .element("CustomMsgRes.GVRes.TANListListRes1")
            .is_some()
    );
    assert!(
        message
            .element("CustomMsgRes.GVRes.TANListListRes1.zustand")
            .is_none()
    );
}

#[test]
fn keeps_resolved_valid_metadata_on_message_elements() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let message = HbciMessage::from_syntax(&syntax, "DialogInit").expect("message tree builds");
    let lang = message
        .element("DialogInit.ProcPrep.lang")
        .expect("language data element exists");

    assert_eq!(lang.valid_values(), ["0", "1", "2", "3"]);
}

#[test]
fn sets_data_element_values_and_exports_java_style_data() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message = HbciMessage::from_syntax(&syntax, "DialogInit").expect("message tree builds");

    message
        .set_value("DialogInit.MsgHead.dialogid", "0")
        .expect("dialog id path exists");
    message
        .set_value("DialogInit.MsgHead.msgnum", "1")
        .expect("message number path exists");
    message
        .set_value("DialogInit.ProcPrep.lang", "0")
        .expect("language path exists");
    message
        .set_value("DialogInit.ProcPrep", "requested")
        .expect("grouping elements accept request tags");

    assert_eq!(message.value("DialogInit.MsgHead.dialogid"), Some("0"));
    assert!(
        message
            .element("DialogInit.ProcPrep")
            .expect("ProcPrep segment exists")
            .is_requested()
    );

    let data = message.data();
    assert_eq!(
        data.get("MsgHead.SegHead.code").map(String::as_str),
        Some("HNHBK")
    );
    assert_eq!(data.get("MsgHead.dialogid").map(String::as_str), Some("0"));
    assert_eq!(data.get("ProcPrep.lang").map(String::as_str), Some("0"));
}

#[test]
fn renders_dialog_end_with_original_delimiters() {
    let mut message = dialog_end_message_with_pintan_signature_shell();

    message
        .prepare_outgoing()
        .expect("message sequences and size are prepared");
    let rendered = message.to_fints_string().expect("message renders");
    let msg_size = message
        .value("DialogEnd.MsgHead.msgsize")
        .expect("message size is set");
    assert_eq!(msg_size, format!("{:012}", rendered.len()));

    assert_eq!(
        rendered,
        format!(
            concat!(
                "HNHBK:1:3+{}+300+DIALOG1+1'",
                "HNSHK:2:4+PIN:1+999+REF1+1+1+1+1+1+1:999:1+6:10:16+280::user:S:1:1'",
                "HKEND:3:1+DIALOG1'",
                "HNSHA:4:2+REF1'",
                "HNHBS:5:1+1'",
            ),
            msg_size,
        )
    );
}

#[test]
fn applies_pintan_usersig_to_signature_tail_like_hbci4java_sig() {
    let mut message = dialog_end_message_with_pintan_signature_shell();
    let signature = UserSig::encode(Some("12345"), Some("987654")).expect("usersig encodes");

    apply_pintan_user_sig_to_sig_tail(&mut message, "DialogEnd.SigTail", &signature)
        .expect("usersig applies to sigtail");

    assert_eq!(
        message.value("DialogEnd.SigTail.UserSig.pin"),
        Some("12345")
    );
    assert_eq!(
        message.value("DialogEnd.SigTail.UserSig.tan"),
        Some("987654")
    );
    assert_eq!(message.value("DialogEnd.SigTail.sig"), None);

    message
        .prepare_outgoing()
        .expect("message sequences and size are prepared");
    let rendered = message.to_fints_string().expect("message renders");
    assert!(
        rendered.contains("HNSHA:4:2+REF1++12345:987654'"),
        "{rendered}"
    );
}

#[test]
fn applies_pintan_usersig_to_signature_tail_without_empty_tan() {
    let mut message = dialog_end_message_with_pintan_signature_shell();
    let signature = UserSig::encode(Some("12345"), None).expect("usersig encodes");

    apply_pintan_user_sig_to_sig_tail(&mut message, "DialogEnd.SigTail", &signature)
        .expect("usersig applies to sigtail");

    assert_eq!(
        message.value("DialogEnd.SigTail.UserSig.pin"),
        Some("12345")
    );
    assert_eq!(message.value("DialogEnd.SigTail.UserSig.tan"), None);

    message
        .prepare_outgoing()
        .expect("message sequences and size are prepared");
    let rendered = message.to_fints_string().expect("message renders");
    assert!(rendered.contains("HNSHA:4:2+REF1++12345'"), "{rendered}");
}

#[test]
fn renders_custom_message_with_single_saldo_gv() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message = HbciMessage::from_syntax(&syntax, "CustomMsg").expect("message tree builds");

    set_all(
        &mut message,
        [
            ("CustomMsg.MsgHead.dialogid", "DIALOG1"),
            ("CustomMsg.MsgHead.msgnum", "1"),
            ("CustomMsg.GV.Saldo7.KTV.iban", "DE02123456780000000000"),
            ("CustomMsg.GV.Saldo7.allaccounts", "N"),
            ("CustomMsg.MsgTail.msgnum", "1"),
        ],
    );

    message
        .prepare_outgoing()
        .expect("message sequences and size are prepared");
    let rendered = message.to_fints_string().expect("message renders");
    let msg_size = message
        .value("CustomMsg.MsgHead.msgsize")
        .expect("message size is set");

    assert_eq!(msg_size, format!("{:012}", rendered.len()));
    assert_eq!(
        rendered,
        format!(
            concat!(
                "HNHBK:1:3+{}+300+DIALOG1+1'",
                "HKSAL:2:7+DE02123456780000000000+N'",
                "HNHBS:3:1+1'",
            ),
            msg_size,
        )
    );
}

#[test]
fn renders_custom_message_with_repeated_saldo_gv() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message = HbciMessage::from_syntax(&syntax, "CustomMsg").expect("message tree builds");

    set_all(
        &mut message,
        [
            ("CustomMsg.MsgHead.dialogid", "DIALOG1"),
            ("CustomMsg.MsgHead.msgnum", "1"),
            ("CustomMsg.GV.Saldo7.KTV.iban", "DE02123456780000000000"),
            ("CustomMsg.GV.Saldo7.allaccounts", "N"),
            ("CustomMsg.GV_2.Saldo7.KTV.iban", "DE02123456780000000001"),
            ("CustomMsg.GV_2.Saldo7.allaccounts", "N"),
            ("CustomMsg.MsgTail.msgnum", "1"),
        ],
    );

    message
        .prepare_outgoing()
        .expect("message sequences and size are prepared");
    let rendered = message.to_fints_string().expect("message renders");
    let msg_size = message
        .value("CustomMsg.MsgHead.msgsize")
        .expect("message size is set");

    assert_eq!(msg_size, format!("{:012}", rendered.len()));
    assert_eq!(
        rendered,
        format!(
            concat!(
                "HNHBK:1:3+{}+300+DIALOG1+1'",
                "HKSAL:2:7+DE02123456780000000000+N'",
                "HKSAL:3:7+DE02123456780000000001+N'",
                "HNHBS:4:1+1'",
            ),
            msg_size,
        )
    );
}

#[test]
fn renders_hktan_challenge_params_with_middle_gaps_like_original_challenge_info_deg_test() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message = HbciMessage::from_syntax(&syntax, "CustomMsg").expect("message tree builds");

    set_all(
        &mut message,
        [
            ("CustomMsg.MsgHead.dialogid", "H11051813102140"),
            ("CustomMsg.MsgHead.msgnum", "3"),
            ("CustomMsg.MsgTail.msgnum", "3"),
            ("CustomMsg.GV.TAN2Step5", "requested"),
            ("CustomMsg.GV.TAN2Step5.process", "1"),
            ("CustomMsg.GV.TAN2Step5.ordersegcode", "HKDAN"),
            ("CustomMsg.GV.TAN2Step5.OrderAccount.number", "12345678"),
            ("CustomMsg.GV.TAN2Step5.OrderAccount.KIK.country", "DE"),
            ("CustomMsg.GV.TAN2Step5.OrderAccount.KIK.blz", "12345678"),
            ("CustomMsg.GV.TAN2Step5.orderhash", "B12345"),
            ("CustomMsg.GV.TAN2Step5.notlasttan", "N"),
            ("CustomMsg.GV.TAN2Step5.challengeklass", "43"),
            ("CustomMsg.GV.TAN2Step5.ChallengeKlassParams.param2", "201,"),
            (
                "CustomMsg.GV.TAN2Step5.ChallengeKlassParams.param3",
                "12345",
            ),
            (
                "CustomMsg.GV.TAN2Step5.ChallengeKlassParams.param5",
                "Param 5",
            ),
        ],
    );

    message
        .prepare_outgoing()
        .expect("message sequences and size are prepared");
    let rendered = message.to_fints_string().expect("message renders");
    let msg_size = message
        .value("CustomMsg.MsgHead.msgsize")
        .expect("message size is set");

    assert_eq!(msg_size, format!("{:012}", rendered.len()));
    assert_eq!(
        rendered,
        "HNHBK:1:3+000000000139+300+H11051813102140+3'\
         HKTAN:2:5+1+HKDAN+::12345678::280:12345678+@5@12345+++N+++43+:201,:12345::Param 5'\
         HNHBS:3:1+3'"
    );
}

#[test]
fn renders_hbci_quoted_data_element_values() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message = HbciMessage::from_syntax(&syntax, "DialogEnd").expect("message tree builds");

    set_all(
        &mut message,
        [
            ("DialogEnd.MsgHead.SegHead.seq", "1"),
            ("DialogEnd.MsgHead.msgsize", "000000000000"),
            ("DialogEnd.MsgHead.dialogid", "DIALOG+1"),
            ("DialogEnd.MsgHead.msgnum", "1"),
        ],
    );

    let rendered = message
        .element("DialogEnd.MsgHead")
        .expect("MsgHead segment exists")
        .to_fints_string()
        .expect("segment renders");

    assert_eq!(rendered, "HNHBK:1:3+000000000000+300+DIALOG?+1+1'");
}

#[test]
fn renders_binary_data_elements_as_length_prefixed_blocks() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message = HbciMessage::from_syntax(&syntax, "Crypted").expect("message tree builds");

    set_all(
        &mut message,
        [
            ("Crypted.CryptData.SegHead.seq", "1"),
            ("Crypted.CryptData.data", "Bpayload+with:delimiters"),
        ],
    );

    let rendered = message
        .element("Crypted.CryptData")
        .expect("crypt data segment exists")
        .to_fints_string()
        .expect("crypt data renders");

    assert_eq!(rendered, "HNVSD:1:1+@23@payload+with:delimiters'");
}

#[test]
fn creates_repeated_message_elements_on_set() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message =
        HbciMessage::from_syntax(&syntax, "DialogEndRes").expect("message tree builds");

    set_all(
        &mut message,
        [
            ("DialogEndRes.RetGlob.SegHead.seq", "1"),
            ("DialogEndRes.RetGlob.RetVal.code", "0010"),
            ("DialogEndRes.RetGlob.RetVal.text", "OK"),
            ("DialogEndRes.RetGlob.RetVal_2.code", "0020"),
            ("DialogEndRes.RetGlob.RetVal_2.text", "Zweite Meldung"),
        ],
    );

    assert_eq!(
        message.value("DialogEndRes.RetGlob.RetVal_2.text"),
        Some("Zweite Meldung")
    );

    let rendered = message
        .element("DialogEndRes.RetGlob")
        .expect("global return segment exists")
        .to_fints_string()
        .expect("return segment renders");

    assert_eq!(rendered, "HIRMG:1:2+0010::OK+0020::Zweite Meldung'");
}

#[test]
fn rejects_repeated_message_elements_beyond_maxnum() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message =
        HbciMessage::from_syntax(&syntax, "DialogEndRes").expect("message tree builds");

    assert!(
        message
            .set_value("DialogEndRes.RetGlob.RetVal_100.code", "9999")
            .is_err()
    );
}

#[test]
fn preserves_defaults_for_repeated_message_segments_created_on_set() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message =
        HbciMessage::from_syntax(&syntax, "DialogEndRes").expect("message tree builds");

    set_all(
        &mut message,
        [
            ("DialogEndRes.RetSeg_2.SegHead.seq", "3"),
            ("DialogEndRes.RetSeg_2.RetVal.code", "0020"),
            ("DialogEndRes.RetSeg_2.RetVal.text", "Zweite Segmentmeldung"),
        ],
    );

    let rendered = message
        .element("DialogEndRes.RetSeg_2")
        .expect("second segment return segment exists")
        .to_fints_string()
        .expect("second segment return segment renders");

    assert_eq!(rendered, "HIRMS:3:2+0020::Zweite Segmentmeldung'");
}

#[test]
fn renders_core_datatypes_like_hbci4java() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message = HbciMessage::from_syntax(&syntax, "DialogEnd").expect("message tree builds");

    set_all(
        &mut message,
        [
            ("DialogEnd.MsgHead.SegHead.seq", "0007"),
            ("DialogEnd.MsgHead.msgsize", "42"),
            ("DialogEnd.MsgHead.dialogid", " DIALOG+1 "),
            ("DialogEnd.MsgHead.msgnum", "0002"),
        ],
    );

    let rendered = message
        .element("DialogEnd.MsgHead")
        .expect("MsgHead segment exists")
        .to_fints_string()
        .expect("segment renders");

    assert_eq!(rendered, "HNHBK:7:3+000000000042+300+DIALOG?+1+2'");
}

#[test]
fn renders_date_and_time_datatypes_through_message_tree() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message = HbciMessage::from_syntax(&syntax, "DialogEnd").expect("message tree builds");

    set_all(
        &mut message,
        [
            ("DialogEnd.SigHead.SecTimestamp.date", "2024-02-29"),
            ("DialogEnd.SigHead.SecTimestamp.time", "07:08:09"),
        ],
    );

    let rendered = message
        .element("DialogEnd.SigHead.SecTimestamp")
        .expect("security timestamp exists")
        .to_fints_string()
        .expect("timestamp renders");

    assert_eq!(rendered, "1:20240229:070809");
}

#[test]
fn rejects_unknown_country_datatype_values() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message = HbciMessage::from_syntax(&syntax, "DialogEnd").expect("message tree builds");

    message
        .set_value("DialogEnd.SigHead.KeyName.KIK.country", "280")
        .expect("country path exists");

    assert!(
        message
            .element("DialogEnd.SigHead.KeyName.KIK")
            .expect("KIK data-element group exists")
            .to_fints_string()
            .is_err()
    );
}

fn set_all<'a>(message: &mut HbciMessage, values: impl IntoIterator<Item = (&'a str, &'a str)>) {
    for (path, value) in values {
        message.set_value(path, value).expect("message path exists");
    }
}

fn dialog_end_message_with_pintan_signature_shell() -> HbciMessage {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let mut message = HbciMessage::from_syntax(&syntax, "DialogEnd").expect("message tree builds");

    set_all(
        &mut message,
        [
            ("DialogEnd.MsgHead.dialogid", "DIALOG1"),
            ("DialogEnd.MsgHead.msgnum", "1"),
            ("DialogEnd.SigHead.SecProfile.method", "PIN"),
            ("DialogEnd.SigHead.SecProfile.version", "1"),
            ("DialogEnd.SigHead.secfunc", "999"),
            ("DialogEnd.SigHead.seccheckref", "REF1"),
            ("DialogEnd.SigHead.role", "1"),
            ("DialogEnd.SigHead.SecIdnDetails.func", "1"),
            ("DialogEnd.SigHead.secref", "1"),
            ("DialogEnd.SigHead.HashAlg.alg", "999"),
            ("DialogEnd.SigHead.SigAlg.alg", "10"),
            ("DialogEnd.SigHead.SigAlg.mode", "16"),
            ("DialogEnd.SigHead.KeyName.KIK.country", "DE"),
            ("DialogEnd.SigHead.KeyName.userid", "user"),
            ("DialogEnd.SigHead.KeyName.keynum", "1"),
            ("DialogEnd.SigHead.KeyName.keyversion", "1"),
            ("DialogEnd.DialogEndS.dialogid", "DIALOG1"),
            ("DialogEnd.SigTail.seccheckref", "REF1"),
            ("DialogEnd.MsgTail.msgnum", "1"),
        ],
    );

    message
}
