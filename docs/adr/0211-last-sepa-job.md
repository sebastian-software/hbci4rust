# ADR 0211: LastSEPA Job

## Status

Accepted

## Context

`GVLastSEPA` submits a SEPA CORE direct debit. It extends
`AbstractGVLastSEPA`, uses the lowlevel job name `LastSEPA`, defaults to
`SepaVersion.PAIN_008_001_01`, and returns `GVRLastSEPA`, whose specialized
payload is only an optional order id.

For HBCI 300 the protocol XML defines `LastSEPA1` / `HKDSE` version 1. The
request contains the creditor account, `sepadescr`, and binary `sepapain`.
`LastSEPARes1` / `HIDSE` version 1 may return `orderid`. hbci4java stores the
submitted lowlevel parameters under `termlast_<orderid>` when the bank returns a
non-empty order id.

The same abstract base also feeds `LastCOR1SEPA`, `LastB2BSEPA`,
`MultiLastSEPA`, and later standing direct-debit jobs. Those jobs need their own
type defaults, segment names, result behavior, or multi-transaction PAIN
handling.

## Decision

Port `LastSEPA` as the first original-near SEPA direct-debit job.

- Use `LastSEPA1` / `HKDSE` version 1 for HBCI 300 requests.
- Use `LastSEPARes1` / `HIDSE` version 1 for response content extraction.
- Add a PAIN.008.001.01 generator for the single CORE direct-debit path,
  matching hbci4java's default `urn:sepade:xsd:pain.008.001.01` descriptor.
- Keep Java-compatible frontend parameters and defaults from
  `AbstractGVLastSEPA` and `GVLastSEPA`, including `type=CORE`,
  `batchbook=0`, `sequencetype=FRST`, `targetdate=1999-01-01`,
  `endtoendid=NOTPROVIDED`, `btg.curr=EUR`, `amendmandindic=false`, and indexed
  debtor/amount/mandate fields.
- Return a typed `GvrLastSepa` result with only `order_id`, mirroring
  `AbstractGVRLastSEPA`.
- Store successful submitted lowlevel data under `termlast_<orderid>` and keep
  the intentionally original key name even though the job is not named
  `TermLastSEPA`.
- Defer `LastCOR1SEPA`, `LastB2BSEPA`, `MultiLast*`, and
  `DauerLastSEPA*` to separate ADRs and commits.

## Consequences

The port gains the root direct-debit PAIN.008 path without broadening the slice
to multi-order or B2B/COR1 behavior. Future direct-debit jobs can reuse the
PAIN.008 generator and the `termlast_` persistence shape, but must record their
own segment and type decisions.
