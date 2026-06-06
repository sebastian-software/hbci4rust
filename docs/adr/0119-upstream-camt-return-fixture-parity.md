# ADR 0119: Upstream CAMT Return Fixture Parity

## Status

Accepted

## Context

ADR 0118 introduced a targeted upstream CAMT fixture for
`TestCamtParse#test004`. The next hbci4java CAMT parser test,
`TestCamtParse#test005`, uses `test-camt-ruecklastschrift.xml` to assert return
transaction behavior.

This fixture exercises the return-side counterparty flip from ADR 0115 with a
real hbci4java document: the entry itself is debit, but the return information
causes hbci4java to map the debtor account/name/BIC as the counterparty. It
also checks the original instructed amount and return additional information.

## Decision

Copy the single upstream fixture
`src/test/resources/org/kapott/hbci4java/sepa/test-camt-ruecklastschrift.xml`
from the pinned reference checkout into
`tests/fixtures/hbci4java/sepa/test-camt-ruecklastschrift.xml` and add a Rust
test for the same observable fields asserted by hbci4java.

Keep this as a targeted fixture copy, not a bulk import of all upstream SEPA
fixtures.

## Consequences

CAMT return parsing now has an upstream regression fixture in addition to the
focused synthetic tests. Future changes to `RtrInf`, counterparty selection, or
original amount mapping can be checked against hbci4java's own return example.
