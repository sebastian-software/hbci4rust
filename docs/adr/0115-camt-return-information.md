# ADR 0115: CAMT Return Information

## Status

Accepted

## Context

ADR 0114 ports the core first-`NtryDtls`/first-`TxDtls` mapping from
hbci4java. The next block in `ParseCamt05200102#createLine(...)` checks
`TxDtls/RtrInf/Rsn/Cd` to detect return transactions.

For such return transactions hbci4java:

- flips the debtor/creditor side used for counterparty mapping;
- copies `AmtDtls/InstdAmt/Amt` into `line.orig_value` when present;
- joins non-empty `RtrInf/AddtlInf` values with commas into
  `line.additional`.

It does not invert the CAMT line amount for this return handling block; the
entry amount and running balance were already derived from `Ntry/Amt` and
`CdtDbtInd`.

## Decision

Port the return-information block into the CAMT parser while preserving the
existing first-detail/first-transaction rule.

Keep the return reason code as an internal detection flag for now, because the
ported result structure has no separate public field for it. Continue mapping
the original instructed amount and additional return information onto the
existing `GvrKUmsLine` fields.

Do not yet port proprietary `BkTxCd/Prtry/Cd` splitting. That remains a
separate CAMT slice.

## Consequences

CAMT return transactions now select the same counterparty side as hbci4java and
preserve the original amount/additional reason text needed by consumers that
display or reconcile returned transfers and direct debits.
