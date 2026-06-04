# ADR 0008: Rust-Native Passport Storage

## Status

Accepted

## Context

hbci4java's newer passport storage uses an AES-based format whose encrypted
payload is Java object serialization. Full Java passport import/export would be
large and brittle.

The v1 scope is PinTAN only, and users can create new Rust-native PinTAN
passports.

## Decision

Do not support Java passport import in v1.

Use a new versioned Rust-native PinTAN passport format. Store a serde JSON
payload inside an encrypted envelope using Argon2id for passphrase-based key
derivation and XChaCha20-Poly1305 for authenticated encryption.

## Consequences

Storage is modern and maintainable, but not Java-compatible. Java passport import
can only be added through a future ADR.

## Links

- `src/passport/storage.rs`
