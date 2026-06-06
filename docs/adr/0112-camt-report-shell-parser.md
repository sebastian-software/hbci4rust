# ADR 0112: CAMT Report Shell Parser

## Status

Accepted

## Context

hbci4java's CAMT parser first creates one `GVRKUms.BTag` per CAMT `Rpt`
element. The report shell maps account identity and the first balance entries
before transaction lines are parsed.

For CAMT.052.001.02, upstream reads:

- `Rpt/Acct/Id/IBAN` into the own account IBAN;
- `Rpt/Acct/Ccy` into the own account currency;
- `Rpt/Acct/Svcr/FinInstnId/BIC` into the own account BIC;
- the first `Bal` as start balance when its code is `PRCD`, `ITBD`, or `OPBD`;
- the second `Bal` as end balance when its code is `CLBD` or `ITBD`;
- debit balances as negative values.

`PRCD` start balance dates are shifted forward by one day, matching
hbci4java's treatment of a previous closing balance as the current opening
balance.

## Decision

Add a CAMT.052 report-shell parser that returns `GvrKUmsBTag` values with
account, start balance, end balance, and empty transaction lines.

Do not parse `Ntry` transaction lines in this slice. Transaction mapping is
larger and includes counterparty, purpose, remittance, return handling, and
balance progression.

## Consequences

CAMT fixtures can now verify the same report-level structure hbci4java builds
before line parsing. Future CAMT transaction slices can extend the parser
without changing the public `GvrKUmsBTag` shape.
