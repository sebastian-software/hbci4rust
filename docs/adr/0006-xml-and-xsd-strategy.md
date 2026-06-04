# ADR 0006: XML And XSD Strategy

## Status

Accepted

## Context

hbci4java uses JAXB-generated Java classes for many PAIN and CAMT XSDs. The Rust
port needs to preserve XML behavior closely without hand-designing a different
domain model too early.

## Decision

Run an XSD-first spike with `xsd-parser` against the original PAIN and CAMT XSDs.

If viable, check generated Rust modules into the repository and use `quick-xml`
for XML serialization/deserialization support. If not viable, fall back to
targeted `quick-xml` streaming parsers/writers and record the fallback in a new
ADR.

## Consequences

The initial plan stays close to JAXB behavior while keeping an explicit escape
hatch if Rust XSD generation is not mature enough for these schemas.

## Links

- `src/sepa/mod.rs`
