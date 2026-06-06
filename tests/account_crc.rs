use hbci4rust::AccountCrcAlgs;

#[test]
fn creditor_id_accepts_original_valid_examples() {
    assert!(AccountCrcAlgs::check_creditor_id("DE98ZZZ09999999999"));
    assert!(AccountCrcAlgs::check_creditor_id("DE09ZZZ00000000001"));
}

#[test]
fn creditor_id_rejects_all_other_check_digits_for_original_fixture() {
    let prefix = "DE";
    let postfix = "ZZZ09999999999";

    for check_digits in 2..98 {
        let candidate = format!("{prefix}{check_digits:02}{postfix}");

        assert!(
            !AccountCrcAlgs::check_creditor_id(&candidate),
            "{candidate}"
        );
    }
}

#[test]
fn creditor_id_keeps_original_de_length_boundary() {
    assert!(!AccountCrcAlgs::check_creditor_id("DE98ZZZ0999999999"));
    assert!(!AccountCrcAlgs::check_creditor_id("DE98ZZZ099999999999"));
    assert!(!AccountCrcAlgs::check_creditor_id("DE98"));
}

#[test]
fn alg_51_accepts_original_test_case_with_null_blz() {
    assert!(AccountCrcAlgs::alg_51(
        None,
        &[0, 0, 0, 2, 6, 7, 1, 0, 7, 1]
    ));
}
