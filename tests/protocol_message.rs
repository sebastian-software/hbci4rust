use hbci4rust::protocol::{HbciMessage, SyntaxElementKind, load_protocol_spec};

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
            ("DialogEnd.MsgHead.dialogid", "DIALOG1"),
            ("DialogEnd.MsgHead.msgnum", "1"),
            ("DialogEnd.SigHead.SegHead.seq", "2"),
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
            ("DialogEnd.SigHead.KeyName.KIK.country", "280"),
            ("DialogEnd.SigHead.KeyName.userid", "user"),
            ("DialogEnd.SigHead.KeyName.keynum", "1"),
            ("DialogEnd.SigHead.KeyName.keyversion", "1"),
            ("DialogEnd.DialogEndS.SegHead.seq", "3"),
            ("DialogEnd.DialogEndS.dialogid", "DIALOG1"),
            ("DialogEnd.SigTail.SegHead.seq", "4"),
            ("DialogEnd.SigTail.seccheckref", "REF1"),
            ("DialogEnd.MsgTail.SegHead.seq", "5"),
            ("DialogEnd.MsgTail.msgnum", "1"),
        ],
    );

    assert_eq!(
        message.to_fints_string().expect("message renders"),
        concat!(
            "HNHBK:1:3+000000000000+300+DIALOG1+1'",
            "HNSHK:2:4+PIN:1+999+REF1+1+1+1+1+1+1:999:1+6:10:16+280::user:S:1:1'",
            "HKEND:3:1+DIALOG1'",
            "HNSHA:4:2+REF1'",
            "HNHBS:5:1+1'",
        )
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

fn set_all<'a>(message: &mut HbciMessage, values: impl IntoIterator<Item = (&'a str, &'a str)>) {
    for (path, value) in values {
        message.set_value(path, value).expect("message path exists");
    }
}
