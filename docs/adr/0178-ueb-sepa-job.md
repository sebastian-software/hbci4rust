# ADR 0178: SEPA Transfer Job

## Status

Accepted

## Context

`GVUebSEPA` is hbci4java's high-level job for a single SEPA credit transfer. It
is an `AbstractSEPAGV` subclass and uses `UebSEPA1` / `HKCCS` with
`pain.001.001.02` as the default PAIN descriptor.

The `UebSEPA1` protocol segment contains only the source account, PAIN
descriptor, and PAIN binary block. Unlike scheduled or standing order jobs, the
plain transfer job has no job-specific response segment or typed result data in
hbci4java; success is determined from the global and segment return values.

ADR 0177 introduced a focused `pain.001.001.02` single-transfer generator. That
is sufficient for a first original-near `UebSEPA` slice.

## Decision

Port `UebSEPA` as the next PinTAN runtime job slice:

- expose hbci4java-like constraints for `UebSEPA1`, including source account
  aliases, `_sepadescriptor`, `_sepapain`, SEPA dummy parameters, `batchbook`,
  `sepaid`, `pmtinfid`, `endtoendid`, and `purposecode`;
- render `UebSEPA1` as `HKCCS` from source account, PAIN descriptor, and
  generated or caller-provided `_sepapain`;
- use the ADR 0177 single-transfer generator when `_sepapain` is absent;
- keep typed result data absent for this slice, matching hbci4java's generic
  `HBCIJobResultImpl` behavior.

Do not port `MultiUebSEPA`, `InstUebSEPA`, `TermUebSEPA`, PAIN.001 versions
newer than `pain.001.001.02`, indexed multi-transfer generation, or BPD-based
PAIN version negotiation in this slice.

## Consequences

The Rust port gains the basic SEPA transfer runtime path and can render an
offline PinTAN request from ordinary hbci4java-like parameters.

The existing PAIN.001 generator becomes useful beyond standing orders while
remaining intentionally narrow.

Future multi-transfer and instant-transfer slices can reuse the same source
account and PAIN rendering shape, but will need separate constraints,
result-parsing rules, and generator extensions.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVUebSEPA.java`
- `target/reference/hbci4java/src/test/java/org/kapott/hbci4java/sepa/TestGVUebSEPA.java`
- `target/reference/hbci4java/src/main/resources/hbci-plus.xml`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `docs/adr/0177-pain001-transfer-generator.md`
