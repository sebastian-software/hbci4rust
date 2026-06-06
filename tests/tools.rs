use std::path::Path;

use hbci4rust::{
    HbciErrorKind, ParameterFinder, ParameterQuery, Properties, has_text, join_strings,
    safe_filename, to_boolean, to_ins_code, to_parameter_code,
};

#[test]
fn string_util_join_joins_values_like_original_test001() {
    assert_eq!(
        join_strings(Some(&["Foo", "Bar", "dong"]), Some(",")),
        Some("Foo,Bar,dong".to_owned())
    );
}

#[test]
fn string_util_join_returns_none_for_null_values_like_original_test002() {
    assert_eq!(join_strings(None, Some(",")), None);
}

#[test]
fn string_util_join_preserves_single_empty_value_like_original_test003() {
    assert_eq!(join_strings(Some(&[""]), Some(",")), Some(String::new()));
}

#[test]
fn string_util_join_uses_empty_separator_for_null_separator_like_original_test004() {
    assert_eq!(
        join_strings(Some(&["foo", "bar"]), None),
        Some("foobar".to_owned())
    );
}

#[test]
fn string_util_hbci_code_helpers_match_original() {
    assert_eq!(to_ins_code(None), None);
    assert_eq!(to_ins_code(Some("HI")), Some("HI".to_owned()));
    assert_eq!(to_ins_code(Some("HKEND")), Some("HIEND".to_owned()));
    assert_eq!(to_parameter_code(None), None);
    assert_eq!(to_parameter_code(Some("HKEND")), Some("HIENDS".to_owned()));
}

#[test]
fn string_util_boolean_and_text_helpers_match_original() {
    assert!(!to_boolean(None));
    assert!(!to_boolean(Some(" false ")));
    assert!(to_boolean(Some(" true ")));
    assert!(!has_text(None));
    assert!(!has_text(Some(" \t\n ")));
    assert!(has_text(Some(" x ")));
}

#[test]
fn io_utils_safe_filename_matches_original_test() {
    assert_eq!(
        safe_filename_name("foobar.txt"),
        Some("foobar.txt".to_owned())
    );
    assert_eq!(
        safe_filename_name("123456789012345678901234567890"),
        Some("1234567890123456789012345".to_owned())
    );
    assert_eq!(
        safe_filename_name("abc&(%$-.txt"),
        Some("abc-.txt".to_owned())
    );
}

#[test]
fn io_utils_safe_filename_handles_null_empty_and_parent_like_original() {
    assert_eq!(safe_filename(None), None);
    assert_eq!(safe_filename(Some("")), Some(String::new()));

    let value = safe_filename(Some("foo/abc&(%$-.txt")).expect("filename is present");
    assert_eq!(
        Path::new(&value).file_name().and_then(|name| name.to_str()),
        Some("abc-.txt")
    );
    assert!(value.contains("foo"));
}

fn safe_filename_name(filename: &str) -> Option<String> {
    let value = safe_filename(Some(filename))?;
    Path::new(&value)
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
}

#[test]
fn parameter_finder_find_collapses_matching_paths_like_original_test001() {
    let props = properties([
        (
            "Params_1.TAN2StepParams1.ParTAN2Step4.TAN2StepParams2.secfunc",
            "a",
        ),
        (
            "Params_2.TAN2StepParamsFoo.ParTAN2Step1.TAN2StepParams1.secfunc",
            "b",
        ),
        (
            "Params_2.TAN2StepParamsFoo.ParTAN2Step.TAN2StepParams.2secfunc",
            "c",
        ),
        (
            "Params.TAN2StepParams1.ParTAN2Step.TAN2StepParams.2secfunc",
            "d1",
        ),
        (
            "Params.TAN2StepParams1.ParTAN2Step.TAN2StepParams.3secfunc",
            "d2",
        ),
        (
            "Params_1.TAN2StepParams1.ParTAN2Step.TAN2StepParams.foo",
            "e",
        ),
        ("Params_1.TAN2StepParams1.ParTAN2Step.secfunc", "f"),
    ]);

    let result = ParameterFinder::find(
        &props,
        Some("Params_*.TAN2StepPar*.ParTAN2Step*.TAN2StepParams*.*secfunc"),
    );

    assert!(result.contains_key("secfunc"));
    assert!(result.contains_key("2secfunc"));
    assert!(!result.contains_key("3secfunc"));
    assert!(!result.contains_key("foo"));
    assert!(matches!(
        result.get("secfunc").map(String::as_str),
        Some("a" | "b")
    ));
    assert_eq!(result.get("2secfunc").map(String::as_str), Some("c"));
}

#[test]
fn parameter_finder_find_all_preserves_matching_paths_like_original_test002() {
    let props = properties([
        (
            "Params_1.TAN2StepParams1.ParTAN2Step4.TAN2StepParams2.secfunc",
            "a",
        ),
        (
            "Params_2.TAN2StepParamsFoo.ParTAN2Step1.TAN2StepParams1.secfunc",
            "b",
        ),
        (
            "Params_2.TAN2StepParamsFoo.ParTAN2Step.TAN2StepParams.2secfunc",
            "c",
        ),
        (
            "Params.TAN2StepParams1.ParTAN2Step.TAN2StepParams.2secfunc",
            "d",
        ),
        (
            "Params_1.TAN2StepParams1.ParTAN2Step.TAN2StepParams.foo",
            "e",
        ),
        ("Params_1.TAN2StepParams1.ParTAN2Step.secfunc", "f"),
    ]);

    let result = ParameterFinder::find_all(
        &props,
        Some("Params_*.TAN2StepPar*.ParTAN2Step*.TAN2StepParams*.*secfunc"),
    );

    assert!(contains_value(&result, "a"));
    assert!(contains_value(&result, "b"));
    assert!(contains_value(&result, "c"));
    assert!(!contains_value(&result, "d"));
    assert!(!contains_value(&result, "e"));
    assert!(!contains_value(&result, "f"));
}

#[test]
fn parameter_finder_known_can1step_query_matches_original_test003() {
    let props = properties([
        ("Params_160.TAN2StepPar1.ParTAN2Step.can1step", "N"),
        (
            "Params_2.TAN2StepParamsFoo.ParTAN2Step.TAN2StepParams.can1step",
            "X",
        ),
        ("Params_161.TAN2StepPar3.ParTAN2Step.can1step", "J"),
    ]);

    let result = ParameterFinder::find_all_query(&props, &ParameterQuery::BPD_PINTAN_CAN1STEP)
        .expect("query parameters are set");

    assert!(contains_value(&result, "J"));
    assert!(contains_value(&result, "N"));
    assert!(!contains_value(&result, "X"));
}

#[test]
fn parameter_finder_parameterized_orderhash_query_matches_original_test004() {
    let props = properties([
        ("Params_160.TAN2StepPar1.ParTAN2Step.orderhashmode", "0"),
        ("Params_161.TAN2StepPar3.ParTAN2Step.orderhashmode", "1"),
        ("Params_162.TAN2StepPar6.ParTAN2Step.orderhashmode", "2"),
    ]);
    let query = ParameterQuery::BPD_PINTAN_ORDERHASHMODE.with_parameters(&["6"]);

    let value =
        ParameterFinder::get_value_query(&props, &query, None).expect("query parameters are set");

    assert_eq!(value.as_deref(), Some("2"));
}

#[test]
fn parameter_finder_rejects_unset_parameterized_query_like_original_test005() {
    let props = Properties::new();

    let err =
        ParameterFinder::get_value_query(&props, &ParameterQuery::BPD_PINTAN_ORDERHASHMODE, None)
            .expect_err("unset query parameters are rejected");

    assert_eq!(err.kind(), HbciErrorKind::InvalidArgument);
    assert!(err.message().contains("Parameters not set in query"));
}

fn properties<const N: usize>(entries: [(&str, &str); N]) -> Properties {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect()
}

fn contains_value(props: &Properties, value: &str) -> bool {
    props.values().any(|candidate| candidate == value)
}
