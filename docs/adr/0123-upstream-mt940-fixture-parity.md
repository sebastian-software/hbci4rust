# ADR 0123: Upstream MT940 Fixture Parity

## Status

Accepted

## Context

The hbci4java offline test class `TestMT940Parse` covers two MT940 fixtures:

- `test-mt940-001.sta` has `:61:` transaction lines with an explicit booking
  date.
- `test-mt940-002.sta` has `:61:` transaction lines without an explicit
  booking date.

Both Java tests read the fixture as ISO-8859-1, append it to `GVRKUms`, flatten
the parsed data, and assert that exactly two lines exist and each line has both
`valuta` and `bdate`.

The Rust port already has hand-written MT940 tests for those parser paths, but
the upstream fixture files were not yet copied into the fixture tree.

## Decision

Copy both upstream MT940 fixtures into `tests/fixtures/hbci4java/swift/` and
add Rust tests that mirror the observable hbci4java assertions.

Decode fixture bytes with a small Latin-1 helper in the test, matching the Java
`StandardCharsets.ISO_8859_1` input boundary.

Keep the assertions intentionally narrow in this slice. More detailed MT940
field parity can be added with separate ADRs once additional hbci4java
expectations or golden outputs are generated.

## Consequences

MT940 parsing now has upstream fixture regression coverage for explicit and
missing booking-date forms. The fixture tree expands beyond SEPA/CAMT while
still preserving the "targeted copies, not vendoring hbci4java" policy.
