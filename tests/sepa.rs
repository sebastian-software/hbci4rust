use hbci4rust::sepa::{
    CAMT_052_001_01_URN, CAMT_052_001_04_URN, CAMT_052_001_07_URN, CAMT_052_001_08_URN, SepaKind,
    SepaVersion,
};

fn camt_document(urn: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="{urn}" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
  <BkToCstmrAcctRpt/>
</Document>"#
    )
}

#[test]
fn camt_version_by_urn_matches_original_known_versions() {
    let version = SepaVersion::by_urn(CAMT_052_001_04_URN).expect("known CAMT version");

    assert_eq!(version, SepaVersion::CAMT_052_001_04);
    assert_eq!(version.kind(), SepaKind::Camt052);
    assert_eq!(version.major(), 1);
    assert_eq!(version.minor(), 4);
    assert_eq!(version.urn(), CAMT_052_001_04_URN);
    assert_eq!(version.schema_file(), Some("camt.052.001.04.xsd"));
    assert_eq!(
        version.schema_location().as_deref(),
        Some("urn:iso:std:iso:20022:tech:xsd:camt.052.001.04 camt.052.001.04.xsd")
    );
}

#[test]
fn camt_version_find_greatest_uses_original_order() {
    let highest = SepaVersion::find_greatest(&[
        SepaVersion::by_urn("urn:iso:std:iso:20022:tech:xsd:camt.052.001.02")
            .expect("known version"),
        SepaVersion::by_urn("urn:iso:std:iso:20022:tech:xsd:camt.052.001.05")
            .expect("known version"),
        SepaVersion::by_urn(CAMT_052_001_07_URN).expect("known version"),
    ]);

    assert_eq!(highest, Some(SepaVersion::CAMT_052_001_07));
}

#[test]
fn camt_version_autodetects_root_namespace() {
    let version = SepaVersion::autodetect(&camt_document(CAMT_052_001_08_URN)).expect("valid XML");

    assert_eq!(version, Some(SepaVersion::CAMT_052_001_08));
}

#[test]
fn camt_version_choose_prefers_xml_data_over_descriptor_like_original() {
    let version = SepaVersion::choose(
        Some(CAMT_052_001_01_URN),
        Some(&camt_document(CAMT_052_001_08_URN)),
    )
    .expect("valid XML");

    assert_eq!(version, Some(SepaVersion::CAMT_052_001_08));
}

#[test]
fn camt_version_choose_falls_back_to_descriptor_when_xml_has_no_namespace() {
    let version = SepaVersion::choose(
        Some(CAMT_052_001_01_URN),
        Some("<Document><BkToCstmrAcctRpt/></Document>"),
    )
    .expect("valid XML");

    assert_eq!(version, Some(SepaVersion::CAMT_052_001_01));
}
