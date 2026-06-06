use hbci4rust::{BankInfo, HbciVersion};

#[test]
fn maps_hbci_versions_by_original_ids() {
    assert_eq!(HbciVersion::by_id("201"), Some(HbciVersion::Hbci201));
    assert_eq!(HbciVersion::by_id("210"), Some(HbciVersion::Hbci210));
    assert_eq!(HbciVersion::by_id("220"), Some(HbciVersion::Hbci220));
    assert_eq!(HbciVersion::by_id("plus"), Some(HbciVersion::HbciPlus));
    assert_eq!(HbciVersion::by_id("300"), Some(HbciVersion::Hbci300));
    assert_eq!(HbciVersion::by_id("400"), Some(HbciVersion::Hbci400));
    assert_eq!(HbciVersion::by_id(""), None);
    assert_eq!(HbciVersion::Hbci300.to_string(), "300: FinTS 3.0");
}

#[test]
fn parses_bank_info_property_value_like_original() {
    let info = BankInfo::parse_property(
        "21070020",
        "Deutsche Bank|Kiel|DEUTDEHH210|63||https://fints.deutsche-bank.de/||300|",
    );

    assert_eq!(info.blz(), Some("21070020"));
    assert_eq!(info.name(), Some("Deutsche Bank"));
    assert_eq!(info.location(), Some("Kiel"));
    assert_eq!(info.bic(), Some("DEUTDEHH210"));
    assert_eq!(info.checksum_method(), Some("63"));
    assert_eq!(info.rdh_address(), Some(""));
    assert_eq!(
        info.pin_tan_address(),
        Some("https://fints.deutsche-bank.de/")
    );
    assert_eq!(info.rdh_version(), None);
    assert_eq!(info.pin_tan_version(), Some(HbciVersion::Hbci300));
    assert_eq!(info.to_string(), "21070020: Deutsche Bank");
}

#[test]
fn drops_trailing_empty_bank_info_columns_like_java_split() {
    let info = BankInfo::parse_value("Commerzbank|Hameln|COBADEFF254|13|||||");

    assert_eq!(info.name(), Some("Commerzbank"));
    assert_eq!(info.location(), Some("Hameln"));
    assert_eq!(info.bic(), Some("COBADEFF254"));
    assert_eq!(info.checksum_method(), Some("13"));
    assert_eq!(info.rdh_address(), None);
    assert_eq!(info.pin_tan_address(), None);
}

#[test]
fn empty_bank_info_matches_original_empty_object_shape() {
    let info = BankInfo::parse_value("");

    assert_eq!(info.blz(), None);
    assert_eq!(info.name(), None);
    assert_eq!(info.to_string(), "null: null");
}
