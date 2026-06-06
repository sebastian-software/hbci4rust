# ADR 0118: Upstream CAMT Fixture Parity

## Status

Accepted

## Context

The original-near port plan calls for porting hbci4java offline tests and
fixtures where they are in scope. The CAMT parser now covers the core
`ParseCamt05200102#createLine(...)` behavior with synthetic focused tests, but
it does not yet exercise an original hbci4java CAMT test document end to end.

The upstream test `TestCamtParse#test004` uses
`test-camt-parse-05200102.xml` and asserts two booked lines with balances,
counterparty fields, proprietary GV metadata, purpose codes, and remittance
text.

## Decision

Copy the single upstream CAMT fixture
`src/test/resources/org/kapott/hbci4java/sepa/test-camt-parse-05200102.xml`
from the pinned reference checkout into
`tests/fixtures/hbci4java/sepa/test-camt-parse-05200102.xml` and assert the
ported Rust output against the same observable fields.

This is a targeted test fixture copy, not vendoring hbci4java source. Keep the
fixture under attribution notes in `tests/fixtures/hbci4java/README.md` and the
project-level `NOTICE`.

Do not yet copy the full hbci4java test resource tree. Additional fixtures
must be pulled in only as their parser behavior becomes in-scope.

## Consequences

CAMT parsing now has a realistic upstream document as an offline regression
anchor. Future CAMT changes can be checked against a stable fixture before
moving on to later CAMT versions and return-specific fixtures.
