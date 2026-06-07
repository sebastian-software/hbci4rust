# ADR 0251: Passport Storage Security Review

## Status

Accepted

## Context

ADR 0008 chose a Rust-native PinTAN passport storage format: serde JSON payload
inside a versioned encrypted envelope using Argon2id for passphrase-based key
derivation and XChaCha20-Poly1305 for authenticated encryption.

ADR 0246 left a v1 release checklist item open to review the KDF, AEAD, and
envelope metadata against current dependency documentation.

The current implementation uses:

- Argon2id version `0x13`;
- memory cost `19 * 1024` KiB;
- time cost `2`;
- parallelism `1`;
- 16-byte random salt;
- 32-byte derived key;
- XChaCha20-Poly1305;
- generated 24-byte XChaCha nonce;
- JSON envelope metadata for format, version, KDF parameters, AEAD name, salt,
  nonce, and ciphertext.

The review checked OWASP's current password-storage guidance, the `argon2`
0.5.3 docs, and the `chacha20poly1305` docs.

## Decision

Keep the current KDF and AEAD choice for v1:

- Argon2id remains the KDF.
- Keep the minimum OWASP-compatible profile `m=19456`, `t=2`, `p=1`.
- Keep XChaCha20-Poly1305 with a 32-byte key and generated 24-byte nonce.
- Keep the envelope as JSON because v1 favors inspectable Rust-native storage
  over Java passport compatibility.

Harden envelope loading before decryption:

- reject empty passphrases on load as well as save;
- reject unsupported AEAD metadata instead of ignoring it;
- reject invalid salt and nonce lengths before deriving/decrypting;
- keep unsupported format, version, KDF, AEAD, and invalid parameter failures as
  `HbciErrorKind::Storage`.

Add tests that pin the envelope metadata, reject metadata tampering, reject
wrong passphrases, and prove obvious plaintext fields are not stored in the
serialized envelope.

## Consequences

The release checklist can treat KDF/AEAD/envelope metadata review as covered for
the current v1 hardening slice.

The storage format remains version `1`; the changes reject malformed envelopes
more explicitly and do not change valid persisted bytes.

This is not a full cryptographic audit. Before the first published release, run
the final package checks and decide whether to add AEAD associated data for
metadata binding or leave metadata validation as the v1 policy.

## Links

- `src/passport/storage.rs`
- `tests/passport.rs`
- `docs/reference/passport-storage-security.md`
- `docs/architecture/release-checklist.md`
- ADR 0008: Rust-Native Passport Storage
- ADR 0246: V1 Release Checklist
- OWASP Password Storage Cheat Sheet:
  `https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html`
- `argon2` crate docs: `https://docs.rs/argon2/latest/argon2/`
- `chacha20poly1305` crate docs:
  `https://docs.rs/chacha20poly1305/latest/chacha20poly1305/`
