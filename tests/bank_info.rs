use hbci4rust::{BankInfo, BankInfoRegistry, HbciVersion};

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

#[test]
fn parses_bank_info_registry_from_properties_text() {
    let registry = BankInfoRegistry::parse_properties(
        "# ignored\n\
         21070020=Deutsche Bank|Kiel|DEUTDEHH210|63||https://fints.deutsche-bank.de/||300|\n\
         25440047:Commerzbank|Hameln|COBADEFF254|13|||||\n\
         ! also ignored\n",
    );

    assert_eq!(registry.len(), 2);
    assert_eq!(registry.name_for_blz("21070020"), "Deutsche Bank");
    assert_eq!(registry.name_for_blz("00000000"), "");

    let info = registry.get_bank_info("25440047").expect("bank info");
    assert_eq!(info.blz(), Some("25440047"));
    assert_eq!(info.name(), Some("Commerzbank"));
    assert_eq!(info.rdh_address(), None);
}

#[test]
fn registry_treats_property_without_separator_as_empty_value() {
    let registry = BankInfoRegistry::parse_properties("12345678\n");

    assert_eq!(registry.len(), 1);
    assert_eq!(registry.name_for_blz("12345678"), "");

    let info = registry.get_bank_info("12345678").expect("bank info");
    assert_eq!(info.blz(), Some("12345678"));
    assert_eq!(info.name(), None);
    assert_eq!(info.to_string(), "12345678: null");
}

#[test]
fn searches_bank_info_like_original() {
    let registry = BankInfoRegistry::parse_properties(
        "30000000=Alpha Bank|Berlin|ALPHDEFF300|00|||||\n\
         10000000=Zeta Bank|Hamburg|ZZZDEHH100|00|||||\n\
         20000000=Deutsche Bank|Kiel|DEUTDEHH200|00|||||\n",
    );

    assert_eq!(registry.search_bank_info("ba"), Vec::<&BankInfo>::new());
    assert_eq!(registry.search_bank_info("  ").len(), 0);

    assert_bank_codes(
        registry.search_bank_info(" bank "),
        &["10000000", "20000000", "30000000"],
    );
    assert_bank_codes(registry.search_bank_info("200"), &["20000000"]);
    assert_bank_codes(registry.search_bank_info("deut"), &["20000000"]);
    assert_bank_codes(registry.search_bank_info("KIE"), &["20000000"]);
    assert_bank_codes(registry.search_bank_info("hamb"), &["10000000"]);
}

fn assert_bank_codes(infos: Vec<&BankInfo>, expected: &[&str]) {
    let actual = infos
        .into_iter()
        .map(|info| info.blz().expect("bank info BLZ"))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected);
}
