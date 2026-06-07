# ADR 0260: Passport Envelope Metadata Binding

## Status

Accepted

## Context

ADR 0008 chose a Rust-native encrypted PinTAN passport envelope for v1. ADR
0251 reviewed the KDF, AEAD, and envelope validation, but left one release
question open: whether v1 should bind envelope metadata through AEAD associated
data or keep the current explicit metadata validation policy.

The current v1 format stores format, version, KDF parameters, AEAD name, salt,
nonce, and ciphertext as JSON fields. Loading validates supported format,
version, KDF, AEAD, salt length, nonce length, and non-empty ciphertext before
decrypting. Salt and KDF parameters derive the decryption key, the nonce is used
by XChaCha20-Poly1305, and ciphertext authentication is verified during
decryption.

Adding associated data now would change the persisted v1 bytes and invalidate
the checked-in v1 storage fixture.

## Decision

Keep v1 without AEAD associated data.

For v1, envelope metadata integrity is handled by:

- rejecting unsupported format, version, KDF, AEAD, salt length, nonce length,
  and empty ciphertext before decryption;
- failing authenticated decryption when valid-looking KDF parameters, salt,
  nonce, or ciphertext bytes are tampered with;
- keeping a checked-in encrypted v1 fixture to pin the persisted format.

Any future move to AEAD associated data must be recorded as a storage format
revision with a new ADR and migration fixture.

## Consequences

The first v1 release candidate does not need a breaking passport storage change.

The metadata is not separately authenticated as associated data, but malformed
or tampered v1 envelopes still cannot load silently: they either fail explicit
validation or fail authenticated decryption.

Future format revisions remain free to add associated data once a migration path
is justified.

## Links

- `src/passport/storage.rs`
- `tests/passport.rs`
- `tests/fixtures/passport/pintan-v1-envelope.json`
- `docs/reference/passport-storage-security.md`
- ADR 0008: Rust-Native Passport Storage
- ADR 0251: Passport Storage Security Review
- ADR 0256: Passport Storage V1 Migration Fixture
