# ADR 0171: Standing Order List Job

## Status

Accepted

## Context

The next PinTAN job slice should extend the port beyond balances, statements,
SEPA info, TAN media, and account information without jumping straight into
live payment submission. hbci4java has an explicit `GVDauerSEPAList` job and
both a GV-level test and a message parsing fixture for `DauerSEPAListRes2`.

`GVDauerSEPAList` is a SEPA standing-order list request. It uses the Java job
name `DauerSEPAList`, the low-level segment family `DauerSEPAList`, and the
result container `GVRDauerList`. The upstream implementation requests source
account data, a SEPA descriptor, optional `orderid`, and optional `maxentries`.
The response embeds a PAIN document and standing-order metadata.

## Decision

Port `DauerSEPAList` as the next original-near read-only PinTAN job slice,
starting with `DauerSEPAList2` / `DauerSEPAListRes2` because the upstream
message fixture and hbci-300 protocol table exercise version 2.

Keep the request side Java-near:

- Public job name remains `DauerSEPAList`.
- Use original frontend parameter names where already supported by the Rust
  API: `my.bic`, `my.iban`, `_sepadescriptor`, `orderid`, and `maxentries`.
- Also accept the original Java test's `src.bic` and `src.iban` aliases for
  this job, mapped to `DauerSEPAList2.My.bic` and `DauerSEPAList2.My.iban`.
- Preserve the default PAIN descriptor from hbci4java's default
  `PAIN_001_001_02`.

Keep the result side incremental but shape-compatible:

- Add a typed `GvrDauerList` result with `GvrDauerListEntry` entries.
- Parse the standing-order envelope fields from `DauerSEPAListRes2` first:
  own account, `sepadescr`, raw `sepapain`, `orderid`, `DauerDetails`,
  `Aussetzung`, and capability flags.
- Keep the raw PAIN payload in the typed result until the SEPA PAIN parser is
  ported far enough to fill beneficiary, amount, usage, payment-info-id, and
  purpose-code with hbci4java parity.
- Preserve `result_data` extraction for all content fields under the response
  root.

Do not implement new, edit, delete, or payment-submission jobs as part of this
slice. Do not persist `dauer_{orderid}` passport data yet; add that only when
the Rust passport has an original-near persistent-data equivalent.

## Consequences

This adds a low-risk read-only job that exercises embedded binary SEPA payloads
and standing-order metadata. It also creates a reusable result type for later
`DauerSEPA*` jobs.

The first implementation will not yet expose every field that hbci4java derives
from the embedded PAIN XML. That gap is intentional and should be closed by a
dedicated SEPA PAIN parser slice rather than ad hoc XML extraction inside the
handler.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVDauerSEPAList.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV_Result/GVRDauerList.java`
- `target/reference/hbci4java/src/test/java/org/kapott/hbci4java/sepa/TestGVDauerSEPAList.java`
- `target/reference/hbci4java/src/test/java/org/kapott/hbci4java/msg/TestDauerSEPAList.java`
- `target/reference/hbci4java/src/test/resources/org/kapott/hbci4java/msg/TestDauerSEPAList.txt`
