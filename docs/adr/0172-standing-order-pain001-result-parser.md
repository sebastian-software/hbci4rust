# ADR 0172: Standing Order PAIN.001 Result Parser

## Status

Accepted

## Context

ADR 0171 added the first `DauerSEPAList2` / `DauerSEPAListRes2` handler slice
and deliberately kept the embedded PAIN document as raw payload. hbci4java does
not stop there: `GVDauerSEPAList.extractResults` calls `SEPAParserFactory` and
copies the first parsed `pain.001` transfer into `GVRDauerList.Dauer`.

The upstream `ParsePain001*` classes all produce Java `Properties` with stable
keys such as `dst.iban`, `dst.bic`, `dst.name`, `value`, `curr`, `usage`,
`pmtinfid`, and sometimes `purposecode`. Older `pain.001.001.02` has slightly
different JAXB types and one usage string, while newer versions may contain
multiple unstructured usage lines and optional purpose codes.

## Decision

Add a Rust-side `pain.001` result parser that is narrow and original-near:

- Parse the same observable fields that `GVDauerSEPAList` consumes:
  beneficiary account/name, amount/currency, usage lines, payment-info-id,
  purpose-code, and requested execution date.
- Use `quick-xml` and local XML names instead of generated PAIN model structs
  for this first parser slice, matching the existing CAMT parser style.
- Make the parser namespace-tolerant and cover both older `pain.001.001.02`
  style data and newer `pain.001.001.09+` style structures.
- Keep only the first parsed transfer in `DauerSEPAList` results for now,
  matching hbci4java's current `sepaResults.get(0)` behavior.

Do not expand the full `SepaVersion` model or implement PAIN generation in this
slice. Those remain separate steps for payment jobs such as `UebSEPA` and
standing-order create/edit jobs.

## Consequences

`DauerSEPAList` becomes more useful and closer to hbci4java without pulling in a
large generated XML model. The parser can also be reused as a stepping stone
for SEPA transfer jobs, but it is not yet a complete PAIN abstraction.

The local-name parser may accept some structurally unusual XML that a generated
schema-bound parser would reject. That is acceptable for this original-near
result extraction slice and should be revisited when the broader PAIN model is
ported.

## Links

- `docs/adr/0171-standing-order-list-job.md`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVDauerSEPAList.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/parsers/ParsePain00100102.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/parsers/ParsePain00100109.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/parsers/ParsePain00100111.java`
