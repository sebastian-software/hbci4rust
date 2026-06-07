use hbci4rust::tools::Properties;
use hbci4rust::{HbciErrorKind, PassportStorage, PinTanPassport, PinTanPassportData, UserSig};
use serde_json::Value;

const V1_FIXTURE_PASSPHRASE: &[u8] = b"hbci4rust-v1-fixture-passphrase";

#[test]
fn usersig_encodes_pin_and_tan_like_hbci4java() {
    let encoded = UserSig::encode(Some("12345"), Some("987654")).expect("usersig encodes");

    assert_eq!(encoded, b"\x0512345987654");
}

#[test]
fn usersig_encodes_java_nulls_as_empty_strings() {
    let encoded = UserSig::encode(None, None).expect("empty usersig encodes");
    let decoded = UserSig::decode(Some(&encoded)).expect("empty usersig decodes");

    assert_eq!(encoded, [0]);
    assert_eq!(decoded.pin(), "");
    assert_eq!(decoded.tan(), "");
}

#[test]
fn usersig_decodes_pin_and_tan_like_hbci4java() {
    let decoded = UserSig::decode(Some(b"\x0512345987654")).expect("usersig decodes");

    assert_eq!(decoded.pin(), "12345");
    assert_eq!(decoded.tan(), "987654");
}

#[test]
fn usersig_decodes_latin1_bytes_like_fints_wire_encoding() {
    let decoded = UserSig::decode(Some(b"\x04M\xfcll4711")).expect("latin1 usersig decodes");

    assert_eq!(decoded.pin(), format!("M{}ll", char::from(0xfc)));
    assert_eq!(decoded.tan(), "4711");
    assert_eq!(
        decoded.to_bytes().expect("latin1 usersig re-encodes"),
        b"\x04M\xfcll4711"
    );
}

#[test]
fn usersig_rejects_missing_or_invalid_signature_bytes() {
    let missing = UserSig::decode(None).expect_err("missing usersig is rejected");
    assert_eq!(missing.kind(), HbciErrorKind::InvalidArgument);

    let too_short = UserSig::decode(Some(b"\x0512")).expect_err("short usersig is rejected");
    assert_eq!(too_short.kind(), HbciErrorKind::InvalidArgument);
}

#[test]
fn usersig_rejects_non_latin1_text() {
    let err =
        UserSig::encode(Some("1234"), Some("\u{1F510}")).expect_err("non-latin1 TAN is rejected");

    assert_eq!(err.kind(), HbciErrorKind::Unsupported);
}

#[test]
fn pintan_passport_caches_and_clears_runtime_pin() {
    let mut passport = PinTanPassport::new(PinTanPassportData::default());

    assert_eq!(passport.pin(), None);

    passport.set_pin("12345");
    assert_eq!(passport.pin(), Some("12345"));

    passport.clear_pin();
    assert_eq!(passport.pin(), None);
}

#[test]
fn pintan_passport_stores_rust_native_persistent_data() {
    let mut passport = PinTanPassport::new(PinTanPassportData::default());
    let data = Properties::from([
        ("DauerDetails.firstdate".to_owned(), "2025-11-01".to_owned()),
        ("sepapain".to_owned(), "<Document/>".to_owned()),
    ]);

    passport.set_persistent_data("dauer_ORDER123", data.clone());

    assert_eq!(
        passport
            .get_persistent_data("dauer_ORDER123")
            .and_then(|data| data.get("DauerDetails.firstdate"))
            .map(String::as_str),
        Some("2025-11-01")
    );
    assert_eq!(
        passport.persistent_data().get("dauer_ORDER123"),
        Some(&data)
    );
    assert_eq!(
        passport.remove_persistent_data("dauer_ORDER123"),
        Some(data)
    );
    assert!(passport.get_persistent_data("dauer_ORDER123").is_none());
}

#[test]
fn passport_storage_envelope_records_reviewed_crypto_metadata() {
    let data = storage_passport_data();
    let bytes = PassportStorage::save_to_vec(&data, b"correct horse battery staple")
        .expect("passport saves");
    let envelope: Value = serde_json::from_slice(&bytes).expect("envelope is JSON");

    assert_eq!(envelope["format"], "hbci4rust-pintan-passport");
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["kdf"]["algorithm"], "argon2id");
    assert_eq!(envelope["kdf"]["memory_cost_kib"], 19 * 1024);
    assert_eq!(envelope["kdf"]["time_cost"], 2);
    assert_eq!(envelope["kdf"]["parallelism"], 1);
    assert_eq!(envelope["aead"], "xchacha20poly1305");
    assert_eq!(envelope["salt"].as_array().expect("salt array").len(), 16);
    assert_eq!(envelope["nonce"].as_array().expect("nonce array").len(), 24);
    assert!(
        !envelope["ciphertext"]
            .as_array()
            .expect("ciphertext array")
            .is_empty()
    );

    let text = String::from_utf8(bytes).expect("envelope is utf-8 JSON");
    assert!(!text.contains("secret-user"));
    assert!(!text.contains("secret-tan-media"));
    assert!(!text.contains("dauer_SECRET"));
}

#[test]
fn passport_storage_loads_persisted_v1_fixture() {
    let bytes = include_bytes!("fixtures/passport/pintan-v1-envelope.json");
    let envelope: Value = serde_json::from_slice(bytes).expect("v1 fixture envelope parses");

    assert_eq!(envelope["format"], "hbci4rust-pintan-passport");
    assert_eq!(envelope["version"], 1);
    assert_eq!(envelope["kdf"]["algorithm"], "argon2id");
    assert_eq!(envelope["kdf"]["memory_cost_kib"], 19 * 1024);
    assert_eq!(envelope["kdf"]["time_cost"], 2);
    assert_eq!(envelope["kdf"]["parallelism"], 1);
    assert_eq!(envelope["aead"], "xchacha20poly1305");
    assert_eq!(envelope["salt"].as_array().expect("salt array").len(), 16);
    assert_eq!(envelope["nonce"].as_array().expect("nonce array").len(), 24);
    assert!(
        !envelope["ciphertext"]
            .as_array()
            .expect("ciphertext array")
            .is_empty()
    );

    let restored = PassportStorage::load_from_slice(bytes, V1_FIXTURE_PASSPHRASE)
        .expect("persisted v1 passport fixture loads");

    assert_eq!(restored.country, "DE");
    assert_eq!(restored.blz, "12345678");
    assert_eq!(
        restored.host.as_deref(),
        Some("https://fints.example.test/fints")
    );
    assert_eq!(restored.user_id, "fixture-user");
    assert_eq!(restored.customer_id.as_deref(), Some("fixture-customer"));
    assert_eq!(restored.filter.as_deref(), Some("Base64"));
    assert_eq!(restored.tan_method.as_deref(), Some("921"));
    assert_eq!(restored.tan_media.as_deref(), Some("fixture-medium"));
    assert_eq!(
        restored.tan_media_names,
        vec!["fixture-medium".to_owned(), "backup-medium".to_owned()]
    );
    assert_eq!(restored.tan_segment_version.as_deref(), Some("5"));
    assert_eq!(restored.bpd_version.as_deref(), Some("6"));
    assert_eq!(restored.upd_version.as_deref(), Some("8"));
    assert_eq!(restored.bank_name.as_deref(), Some("Fixture Bank"));
    assert_eq!(
        restored.supported_hbci_versions,
        vec!["300".to_owned(), "220".to_owned()]
    );
    assert_eq!(
        restored
            .persistent_data
            .get("dauer_FIXTURE")
            .and_then(|data| data.get("DauerDetails.firstdate"))
            .map(String::as_str),
        Some("2026-01-02")
    );

    let text = std::str::from_utf8(bytes).expect("fixture is utf-8 JSON");
    assert!(!text.contains("fixture-user"));
    assert!(!text.contains("fixture-medium"));
    assert!(!text.contains("dauer_FIXTURE"));
}

#[test]
fn passport_storage_rejects_empty_passphrases_on_save_and_load() {
    let data = storage_passport_data();
    let bytes = PassportStorage::save_to_vec(&data, b"correct horse battery staple")
        .expect("passport saves");

    let save_err =
        PassportStorage::save_to_vec(&data, b"").expect_err("empty save passphrase is rejected");
    let load_err = PassportStorage::load_from_slice(&bytes, b"")
        .expect_err("empty load passphrase is rejected");

    assert_eq!(save_err.kind(), HbciErrorKind::InvalidArgument);
    assert_eq!(load_err.kind(), HbciErrorKind::InvalidArgument);
}

#[test]
fn passport_storage_rejects_wrong_passphrase_and_tampered_metadata() {
    let data = storage_passport_data();
    let bytes = PassportStorage::save_to_vec(&data, b"correct horse battery staple")
        .expect("passport saves");

    let wrong_passphrase = PassportStorage::load_from_slice(&bytes, b"wrong passphrase")
        .expect_err("wrong passphrase cannot decrypt");
    assert_eq!(wrong_passphrase.kind(), HbciErrorKind::Storage);
    assert_eq!(wrong_passphrase.message(), "failed to decrypt passport");

    let mut envelope: Value = serde_json::from_slice(&bytes).expect("envelope parses");

    envelope["ciphertext"][0] =
        Value::from(envelope["ciphertext"][0].as_u64().expect("ciphertext byte") ^ 1);
    let err = PassportStorage::load_from_slice(
        &serde_json::to_vec(&envelope).expect("tampered envelope serializes"),
        b"correct horse battery staple",
    )
    .expect_err("ciphertext tampering is rejected");
    assert_eq!(err.kind(), HbciErrorKind::Storage);
    assert_eq!(err.message(), "failed to decrypt passport");

    let mut envelope: Value = serde_json::from_slice(&bytes).expect("envelope parses");

    envelope["kdf"]["time_cost"] = Value::from(1);
    let err = PassportStorage::load_from_slice(
        &serde_json::to_vec(&envelope).expect("tampered envelope serializes"),
        b"correct horse battery staple",
    )
    .expect_err("valid-looking KDF metadata tampering is rejected");
    assert_eq!(err.kind(), HbciErrorKind::Storage);
    assert_eq!(err.message(), "failed to decrypt passport");

    let mut envelope: Value = serde_json::from_slice(&bytes).expect("envelope parses");

    envelope["aead"] = Value::String("aes-gcm".to_owned());
    let err = PassportStorage::load_from_slice(
        &serde_json::to_vec(&envelope).expect("tampered envelope serializes"),
        b"correct horse battery staple",
    )
    .expect_err("unsupported AEAD is rejected");
    assert_eq!(err.kind(), HbciErrorKind::Storage);
    assert_eq!(err.message(), "unsupported passport AEAD");

    envelope["aead"] = Value::String("xchacha20poly1305".to_owned());
    envelope["nonce"] = Value::Array(Vec::new());
    let err = PassportStorage::load_from_slice(
        &serde_json::to_vec(&envelope).expect("tampered envelope serializes"),
        b"correct horse battery staple",
    )
    .expect_err("invalid nonce is rejected before decrypt");
    assert_eq!(err.kind(), HbciErrorKind::Storage);
    assert_eq!(err.message(), "invalid passport nonce length");

    envelope["nonce"] =
        serde_json::from_slice::<Value>(&bytes).expect("envelope parses")["nonce"].clone();
    envelope["kdf"]["algorithm"] = Value::String("pbkdf2".to_owned());
    let err = PassportStorage::load_from_slice(
        &serde_json::to_vec(&envelope).expect("tampered envelope serializes"),
        b"correct horse battery staple",
    )
    .expect_err("unsupported KDF is rejected");
    assert_eq!(err.kind(), HbciErrorKind::Storage);
    assert_eq!(err.message(), "unsupported passport KDF");
}

fn storage_passport_data() -> PinTanPassportData {
    PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "secret-user".to_owned(),
        customer_id: Some("secret-customer".to_owned()),
        tan_media: Some("secret-tan-media".to_owned()),
        persistent_data: std::collections::BTreeMap::from([(
            "dauer_SECRET".to_owned(),
            Properties::from([("DauerDetails.firstdate".to_owned(), "2026-01-02".to_owned())]),
        )]),
        ..PinTanPassportData::default()
    }
}
