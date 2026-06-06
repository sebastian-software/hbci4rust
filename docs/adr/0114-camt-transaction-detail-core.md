# ADR 0114: CAMT Transaction Detail Core

## Status

Accepted

## Context

ADR 0113 ports basic `Rpt/Ntry` line creation. hbci4java then reads the first
`NtryDtls` element and the first `TxDtls` element. If the selected detail has
no transaction details, `ParseCamt05200102#createLine(...)` returns no line.

The next original-near fields in that first transaction detail are:

- proprietary/account-servicer/end-to-end/mandate references;
- related debtor or creditor account, name, ultimate name, and agent BIC,
  selected by the entry credit/debit indicator;
- unstructured remittance lines;
- purpose code.

hbci4java also maps creditor IDs, return information, instructed original
amounts, return reasons, and proprietary bank transaction-code fragments in
the same method.

## Decision

Port the core first-`TxDtls` mapping now and keep the parser behavior close to
hbci4java: use only the first detail and first transaction detail, skip entries
whose first detail contains no transaction detail, and keep existing entry-level
amount/date/balance behavior.

Add the missing Rust-native `Konto::creditorid` field because the CAMT parser
has an upstream target for it. Keep `Konto` display and equality behavior
unchanged, matching hbci4java's existing non-use of `creditorid` in those
operations.

Do not yet port return handling, original instructed amounts, return reasons,
or proprietary `BkTxCd/Prtry/Cd` splitting. Those stay separate follow-up
slices.

## Consequences

CAMT line parsing now carries the central SEPA transaction identifiers,
counterparty data, remittance text, and purpose code for simple detailed CAMT
reports. Follow-up slices can focus on special cases without changing the
first-detail/first-transaction selection rule again.
