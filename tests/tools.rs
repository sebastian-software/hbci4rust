use std::path::Path;

use hbci4rust::{
    has_text, join_strings, safe_filename, to_boolean, to_ins_code, to_parameter_code,
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
