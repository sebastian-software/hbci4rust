# ADR 0113: CAMT Basic Entry Lines

## Status

Accepted

## Context

ADR 0112 added CAMT report-shell parsing: own account, start balance, and end
balance. hbci4java then iterates `Rpt/Ntry` entries and creates an `UmsLine`
before reading transaction details.

The first part of upstream `ParseCamt05200102#createLine(...)` maps:

- amount and currency;
- `CdtDbtInd` into the amount sign;
- `RvslInd` into the cancellation flag;
- booking and value dates, with each falling back to the other when absent;
- running balance from the current balance plus the entry amount;
- `AddtlNtryInf` into line text;
- `AcctSvcrRef` into customer reference.

When an entry has no `NtryDtls`, hbci4java additionally copies `AddtlNtryInf`
into the usage list and returns the line.

## Decision

Port this basic CAMT entry-line layer into the current CAMT report parser.

Do not yet parse `NtryDtls/TxDtls`, counterparty data, proprietary bank
transaction codes, purpose codes, return information, or remittance details.
Those are separate follow-up slices.

## Consequences

CAMT report parsing now yields useful `GvrKUmsLine` entries for simple reports
and establishes the running-balance behavior needed by later detail mapping.
