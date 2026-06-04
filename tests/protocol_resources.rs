use hbci4rust::protocol::{HBCI_DTD, load_protocol_spec};

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
