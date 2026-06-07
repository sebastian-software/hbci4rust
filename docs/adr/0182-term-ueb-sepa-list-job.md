# ADR 0182: Scheduled SEPA Transfer List Job

## Status

Accepted

## Context

`GVTermUebSEPAList` is hbci4java's high-level job for listing existing
scheduled SEPA credit transfers. It is an `AbstractSEPAGV` subclass and uses
`TermUebSEPAList1` / `HKCSB` with `pain.001.001.02` as the default PAIN
descriptor.

The request segment contains the source account, one or more PAIN descriptors,
and optional `startdate`, `enddate`, `maxentries`, and protocol-level `offset`.
hbci4java exposes `src.bic` and `src.iban`, while the upstream online test sets
`my.bic` and `my.iban`. The existing Rust `DauerSEPAList` port already accepts
both source-account aliases for this reason.

The response segment `TermUebSEPAListRes1` / `HICSB` contains the source
account, PAIN descriptor, embedded PAIN binary block, optional `orderid`, and
optional `candel` / `canchange` flags. hbci4java parses the first PAIN.001
transfer into `GVRTermUebList.Entry`, derives the execution date from the PAIN
payload, and persists request data under `termueb_{orderid}` when an order id
is present.

## Decision

Port `TermUebSEPAList` as the next scheduled-transfer slice:

- expose original-near constraints for `TermUebSEPAList1`, including `my.*`
  and `src.*` source-account aliases, `_sepadescriptor`, `startdate`,
  `enddate`, and `maxentries`;
- render `TermUebSEPAList1` as `HKCSB`, preserving the segment order account,
  descriptor, date range, and max entries;
- add a typed `GvrTermUebList` / `GvrTermUebListEntry` result with the fields
  used by hbci4java's `GVRTermUebList.Entry`: own account, counterparty,
  value, usage, execution date, order id, can-change, can-delete,
  `sepadescr`, and raw `sepapain`;
- reuse the ADR 0172 PAIN.001 result parser and keep the first parsed transfer,
  matching hbci4java's `sepaResults.get(0)` behavior;
- persist a `termueb_{orderid}` snapshot from response content data when the
  response contains a non-empty order id, excluding `SegHead.*` and `orderid`.

Do not port non-SEPA `TermUebList`, `TermMultiUebSEPA`, protocol-level
`offset`, pagination, newer PAIN version negotiation, or BPD-driven
`canmaxentries` / `cantimerange` behavior in this slice.

## Consequences

The Rust port can now replay-test listing scheduled SEPA transfers and reuse
the resulting persistent snapshot for later scheduled transfer edit/delete
jobs.

The typed result is intentionally separate from `GvrDauerList`, even though the
PAIN parsing code is shared. This keeps the public Rust API close to
hbci4java's separate `GVRTermUebList` result while avoiding premature
abstraction over all scheduled-payment result shapes.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVTermUebSEPAList.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRTermUebList.java`
- `target/reference/hbci4java/src/main/resources/hbci-plus.xml`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `docs/adr/0172-standing-order-pain001-result-parser.md`
- `docs/adr/0181-term-ueb-sepa-delete-job.md`
