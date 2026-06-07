# Passport Storage Security Review

Snapshot date: 2026-06-07.

This page records the current security review for the v1 Rust-native PinTAN
passport storage format. It is a release-hardening note, not a full external
cryptographic audit.

## Format

v1 stores `PinTanPassportData` as serde JSON plaintext, then encrypts that JSON
inside a versioned envelope:

- `format`: `hbci4rust-pintan-passport`
- `version`: `1`
- `kdf`: Argon2id parameters
- `aead`: `xchacha20poly1305`
- `salt`: random 16-byte salt
- `nonce`: random 24-byte XChaCha nonce
- `ciphertext`: encrypted passport payload plus authentication tag

Java passport import/export remains outside v1. This format is Rust-native and
versioned for future migration.

## KDF

The implementation derives a 32-byte key with:

- algorithm: Argon2id
- version: `0x13`
- memory cost: `19456` KiB
- time cost: `2`
- parallelism: `1`

This matches OWASP's current minimum Argon2id profile and the `argon2` 0.5.3
default cost constants. The code stores these KDF parameters in the envelope and
uses them when loading, so future format versions can raise the cost while still
reading older version-1 files.

## AEAD

The implementation uses `XChaCha20Poly1305` from `chacha20poly1305` 0.10.1 with
a 32-byte key and generated 24-byte nonce. The crate documents XChaCha20Poly1305
as the ChaCha20Poly1305 variant with an extended 192-bit nonce.

The loader now rejects unsupported AEAD metadata instead of ignoring it.
ADR 0260 keeps v1 without AEAD associated data. Instead, v1 relies on explicit
metadata validation before decryption plus authenticated decryption failure for
valid-looking tampering of KDF parameters, nonce, salt, or ciphertext.

## Envelope Validation

Before decryption, the loader checks:

- non-empty passphrase;
- supported envelope format and version;
- supported AEAD name;
- 16-byte salt length;
- 24-byte nonce length;
- non-empty ciphertext;
- supported KDF name and valid Argon2 parameters.

Wrong passphrases and ciphertext tampering fail during authenticated
decryption.

## Tests

`tests/passport.rs` pins the reviewed shape:

- envelope metadata values;
- loading of a checked-in version-1 encrypted envelope fixture;
- salt and nonce lengths;
- rejection of empty passphrases on save and load;
- rejection of wrong passphrases;
- rejection of unsupported AEAD and KDF metadata;
- rejection of invalid nonce length before decryption;
- rejection of ciphertext and valid-looking KDF parameter tampering during
  authenticated decryption;
- absence of obvious plaintext passport fields in the serialized envelope.

The persisted v1 fixture lives at
`tests/fixtures/passport/pintan-v1-envelope.json`. It uses only dummy values and
the dummy passphrase `hbci4rust-v1-fixture-passphrase`.

## Future Format Work

- keep the checked-in v1 fixture loading or add explicit migration fixtures
  before changing the envelope version;
- record a new ADR before adding AEAD associated data or otherwise changing
  valid persisted bytes.

Reviewed sources:

- OWASP Password Storage Cheat Sheet:
  `https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html`
- `argon2` 0.5.3 docs:
  `https://docs.rs/argon2/0.5.3/argon2/`
- `chacha20poly1305` 0.10.1 docs:
  `https://docs.rs/chacha20poly1305/0.10.1/chacha20poly1305/`
- ADR 0260
