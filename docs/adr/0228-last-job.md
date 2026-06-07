# ADR 0228: Last Job

## Status

Superseded by ADR 0267 and ADR 0273

## Context

`GVLast` in hbci4java implements the classic domestic direct debit job. Its
lowlevel name is `Last`; in FinTS 3.0 this maps to request segment `Last5` /
`HKLAS` version 5.

The job uses the same `SingleInlandUser4` protocol shape as classic domestic
transfers, but its frontend parameter names differ from `GVUeb`:

- creditor/source account fields use `my.country`, `my.blz`, `my.number`, and
  `my.subnumber`;
- debtor/counterparty account fields use `other.country`, `other.blz`,
  `other.number`, and `other.subnumber`;
- amount fields are `btg.value` and `btg.curr`;
- name fields are `name` and `name2`;
- the transaction key is exposed as `type` and mapped to lowlevel `key`,
  defaulting to `"05"`;
- usage lines keep the original `usage`, `usage_2`, ... naming pattern from
  the job restrictions.

hbci4java verifies account CRCs for both `my` and `other`. The protocol does not
define a `LastRes` response segment for successful submissions; hbci4java uses
the generic `HBCIJobResultImpl` result shape and relies on status segments.

## Decision

Port `Last` as an original-near legacy domestic PinTAN job.

- Add `Last` to the PinTAN job registry.
- Map constraints to `Last5`, preserving the hbci4java frontend names,
  especially `type -> Last5.key`.
- Render `Last5` / `HKLAS` with a narrow renderer that reuses the existing
  national account helpers but keeps `my` and `other` frontend bases.
- Add the job to the PinTAN order-hash mapping with segment code `HKLAS`.
- Keep result handling generic and do not invent a typed result or content
  parser.

## Consequences

Callers can create `new_job("Last")` with hbci4java-style parameter keys. The
implementation advances legacy domestic job parity without changing the SEPA
direct-debit implementation or recommending this job for modern live-bank use.

## References

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVLast.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
