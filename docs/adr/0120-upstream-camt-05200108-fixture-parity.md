# ADR 0120: Upstream CAMT 052.001.08 Fixture Parity

## Status

Accepted

## Context

ADR 0118 and ADR 0119 introduced targeted hbci4java CAMT fixtures for
CAMT.052.001.02 booked transactions and return transactions. The next upstream
CAMT parser test, `TestCamtParse#test006`, uses
`test-camt-parse-05200108.xml` to exercise a CAMT.052.001.08 direct debit.

Compared with the CAMT.052.001.02 fixtures, this document uses newer party
choice wrappers such as `RltdPties/Cdtr/Pty/Nm` and
`RltdPties/Cdtr/Pty/Id/PrvtId/Othr/Id`. The existing parser is intentionally
local-name based, but it currently only recognizes the older direct
`Cdtr/Nm` and `Cdtr/Id/...` shapes.

## Decision

Copy the single upstream fixture
`src/test/resources/org/kapott/hbci4java/sepa/test-camt-parse-05200108.xml`
from the pinned reference checkout into
`tests/fixtures/hbci4java/sepa/test-camt-parse-05200108.xml` and add a Rust
test for the observable fields asserted by hbci4java.

Extend CAMT transaction-detail text collection to accept the party choice
wrapper variants for debtor and creditor names and creditor IDs while keeping
the existing first-detail/first-transaction rule.

Keep this as a targeted fixture copy and parser compatibility slice, not a
bulk import of all upstream SEPA fixtures.

## Consequences

The CAMT parser gains an offline regression anchor for CAMT.052.001.08 and a
small version-tolerant mapping improvement for party fields used by newer CAMT
schemas.
