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
