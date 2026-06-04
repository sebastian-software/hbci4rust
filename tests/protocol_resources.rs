use hbci4rust::protocol::{DefinitionKind, HBCI_DTD, SyntaxChildKind, load_protocol_spec};

#[test]
fn loads_original_protocol_specs() {
    for version in ["201", "210", "220", "300", "plus"] {
        let spec = load_protocol_spec(version).expect("known protocol version loads");

        assert_eq!(spec.version, version);
        assert!(spec.xml.starts_with("<?xml version=\"1.0\"?>"));
        assert!(
            spec.msg_definition_count().expect("MSGdefs parse") > 0,
            "{version} should contain message definitions",
        );
        assert!(
            spec.seg_definition_count().expect("SEGdefs parse") > 0,
            "{version} should contain segment definitions",
        );
        assert!(
            spec.deg_definition_count().expect("DEGdefs parse") > 0,
            "{version} should contain data-element-group definitions",
        );
    }
}

#[test]
fn exposes_upstream_hbci_dtd() {
    assert!(HBCI_DTD.contains("<!ELEMENT hbci"));
    assert!(HBCI_DTD.contains("<!ELEMENT MSGdef"));
}

#[test]
fn parses_protocol_syntax_definitions_by_id() {
    let spec = load_protocol_spec("300").expect("known protocol version loads");
    let syntax = spec.parse_syntax().expect("syntax parses");

    assert_eq!(syntax.version(), "300");
    assert_eq!(
        syntax.definition_count(DefinitionKind::Deg),
        spec.deg_definition_count().expect("DEG count parses"),
    );
    assert_eq!(
        syntax.definition_count(DefinitionKind::Seg),
        spec.seg_definition_count().expect("SEG count parses"),
    );
    assert_eq!(
        syntax.definition_count(DefinitionKind::Sf),
        spec.sf_definition_count().expect("SF count parses"),
    );
    assert_eq!(
        syntax.definition_count(DefinitionKind::Msg),
        spec.msg_definition_count().expect("MSG count parses"),
    );

    let dialog_init = syntax
        .definition("DialogInit")
        .expect("DialogInit MSGdef exists");
    assert_eq!(dialog_init.kind, DefinitionKind::Msg);
    assert_eq!(dialog_init.children[0].kind, SyntaxChildKind::Seg);
    assert_eq!(dialog_init.children[0].type_name, "MsgHeadUser");
    assert_eq!(dialog_init.children[1].kind, SyntaxChildKind::Seg);
    assert_eq!(dialog_init.children[1].type_name, "SigHeadUser");
    assert!(
        dialog_init
            .children
            .iter()
            .any(|child| child.kind == SyntaxChildKind::Seg && child.type_name == "Idn")
    );
}

#[test]
fn parses_child_refs_and_default_values() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let msg_head = syntax
        .definition("MsgHeadUser")
        .expect("MsgHeadUser SEGdef exists");
    assert_eq!(msg_head.kind, DefinitionKind::Seg);
    assert_eq!(msg_head.children[0].kind, SyntaxChildKind::Deg);
    assert_eq!(msg_head.children[0].type_name, "SegHeadUser");
    assert_eq!(msg_head.children[0].name.as_deref(), Some("SegHead"));

    assert!(
        msg_head
            .values
            .iter()
            .any(|value| { value.path == "SegHead.code" && value.value == "HNHBK" })
    );
    assert!(
        msg_head
            .values
            .iter()
            .any(|value| { value.path == "hbciversion" && value.value == "300" })
    );
}

#[test]
fn resolves_segment_definitions_by_code_and_version() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let ret_glob = syntax
        .segment_definition("HIRMG", "2")
        .expect("HIRMG version 2 segment exists");
    assert_eq!(ret_glob.id, "RetGlob");
    assert_eq!(ret_glob.segment_code(), Some("HIRMG"));
    assert_eq!(ret_glob.segment_version(), Some("2"));

    let upd_ids = syntax
        .segment_definitions_by_code("HIUPD")
        .map(|definition| definition.id.as_str())
        .collect::<Vec<_>>();
    assert!(upd_ids.contains(&"KInfo5"));
    assert!(upd_ids.contains(&"KInfo6"));
    assert_eq!(
        syntax
            .segment_definition("HIUPD", "5")
            .expect("HIUPD version 5 segment exists")
            .id,
        "KInfo5"
    );
    assert!(syntax.segment_definition("HIUPD", "9").is_none());

    let dialog_init = syntax
        .definition("DialogInit")
        .expect("DialogInit MSGdef exists");
    assert_eq!(dialog_init.kind, DefinitionKind::Msg);
    assert_eq!(dialog_init.segment_code(), None);
    assert_eq!(dialog_init.segment_version(), None);
}

#[test]
fn parses_and_expands_dtd_entities() {
    let syntax = load_protocol_spec("300")
        .expect("known protocol version loads")
        .parse_syntax()
        .expect("syntax parses");

    let msg_sig_head_user = syntax
        .entity("MsgSigHeadUser")
        .expect("MsgSigHeadUser entity exists");
    assert_eq!(msg_sig_head_user.children.len(), 2);
    assert_eq!(msg_sig_head_user.children[0].kind, SyntaxChildKind::Seg);
    assert_eq!(msg_sig_head_user.children[0].type_name, "MsgHeadUser");
    assert_eq!(
        msg_sig_head_user.children[0].name.as_deref(),
        Some("MsgHead")
    );
    assert_eq!(msg_sig_head_user.children[1].kind, SyntaxChildKind::Seg);
    assert_eq!(msg_sig_head_user.children[1].type_name, "SigHeadUser");
    assert_eq!(msg_sig_head_user.children[1].max_num.as_deref(), Some("3"));

    let sec_class_valids = syntax
        .entity("SecClassValids")
        .expect("SecClassValids entity exists");
    assert_eq!(sec_class_valids.valids.len(), 1);
    assert_eq!(sec_class_valids.valids[0].path, "secclass");
    assert_eq!(sec_class_valids.valids[0].values, ["0", "1", "2", "3", "4"]);

    assert!(syntax.definitions().all(|definition| {
        definition
            .children
            .iter()
            .all(|child| child.kind != SyntaxChildKind::EntityRef)
    }));
}
