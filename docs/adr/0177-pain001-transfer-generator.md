# ADR 0177: PAIN.001 Transfer Generator

## Status

Accepted

## Context

hbci4java's `AbstractSEPAGV.verifyConstraints()` generates `_sepapain` before
the normal constraint verification whenever a job accepts `_sepapain`. For the
standing order jobs ported in ADR 0174, ADR 0175, and ADR 0176, the Rust port
temporarily required callers to provide the PAIN XML directly.

The upstream generator used by these standing order transfer jobs is
`GenUebSEPA00100102`. It builds a `pain.001.001.02` credit transfer document
from job-like `Properties`, sets `GrpHdr.MsgId` from `sepaid`, uses
`pmtinfid` or falls back to `sepaid`, defaults the execution date to
`SepaUtil.DATE_UNDEFINED` (`1999-01-01`), defaults an empty end-to-end id to
`NOTPROVIDED`, and uses `EUR` when `btg.curr` is empty.

## Decision

Add a focused Rust generator for `pain.001.001.02` single credit transfers:

- keep the input surface property-near, using the existing Java parameter keys
  such as `src.iban`, `dst.name`, `btg.value`, `usage`, `sepaid`, and
  `endtoendid`;
- render XML with the existing `quick-xml` dependency instead of introducing
  JAXB-like generated Rust structs in this slice;
- support unindexed single-transfer data first, because that is what
  `DauerSEPANew`, `DauerSEPAEdit`, and `DauerSEPADel` need;
- generate `_sepapain` during `HbciJob::verify_constraints()` for the SEPA
  standing order jobs when callers did not already provide `_sepapain`;
- preserve caller-provided `_sepapain` unchanged, so existing replay fixtures
  and low-level escape-hatch tests continue to work.

Do not port indexed multi-transfer generation, schema validation, PAIN version
selection from BPD restrictions, or PAIN.008 generators in this slice.

## Consequences

The standing order transfer jobs can now be queued from ordinary hbci4java-like
SEPA parameters instead of requiring prebuilt PAIN XML.

The generator is deliberately narrower than hbci4java's full
`SEPAGeneratorFactory`. Future slices can add indexed transactions and other
PAIN versions without changing the public job parameter keys introduced here.

Checking in a hand-written generator slightly increases local XML-rendering
code, but avoids a larger XSD-codegen step before we have broader generator
coverage.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/AbstractSEPAGV.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/SepaUtil.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/generators/GenUebSEPA00100102.java`
- `docs/adr/0174-standing-order-sepa-new-job.md`
- `docs/adr/0175-standing-order-sepa-edit-job.md`
- `docs/adr/0176-standing-order-sepa-delete-job.md`
