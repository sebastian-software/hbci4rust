# ADR 0121: Upstream CAMT 052.001.08 Tolerance Fixtures

## Status

Accepted

## Context

ADR 0120 added the main hbci4java CAMT.052.001.08 fixture for
`TestCamtParse#test006`. The next two upstream CAMT parser tests,
`TestCamtParse#test007` and `TestCamtParse#test008`, use closely related
CAMT.052.001.08 documents and assert that parsing completes without throwing.

The fixtures cover tolerance boundaries observed in real bank data:

- `test-camt-parse-5200108-missing-date.xml` omits the balance dates.
- `test-camt-parse-5200108-invalid-saldo.xml` uses balance amounts with more
  fractional digits than normal account balances.

## Decision

Copy both upstream fixtures into
`tests/fixtures/hbci4java/sepa/` and add Rust tests that mirror the observable
hbci4java contract: autodetect the CAMT.052.001.08 version and accept the
document through `parse_camt_report_shell`.

Keep the tests as parser tolerance regressions rather than expanding them into
new semantic assertions. The corresponding hbci4java tests only prove that the
parser accepts those documents, so stronger expectations should be introduced
only by a separate ADR if upstream behavior is inspected further.

## Consequences

The CAMT parser now has offline regression coverage for the remaining
CAMT.052.001.08 parser tolerance tests in `TestCamtParse`. Future balance-date
or decimal-normalization changes can be checked against these fixtures without
depending on live bank data.
