# ADR 0199: TermUebList Job

## Status

Superseded by ADR 0269 and ADR 0276

## Context

`GVTermUebList` queries pending classic, non-SEPA scheduled transfers. It is the
classic counterpart to the already ported `TermUebSEPAList` job and returns
hbci4java's `GVRTermUebList`.

For HBCI 300 the protocol XML provides `TermUebList1` through `TermUebList3`.
Version 3 uses `KTV3` for the request account and `SingleInlandInst4` for the
response. The Java job constructor adds constraints for national account data,
optional date range filters, and optional `maxentries`. It also checks the
account CRC for `my`.

`SingleInlandInst4` includes an optional `status` value, but hbci4java's
`GVTermUebList.extractResults` does not expose it on `GVRTermUebList.Entry`.

## Decision

Port `TermUebList` as an original-near classic scheduled-transfer query job.

- Use `TermUebList3` / `HKTUB` version 3 for HBCI 300 requests and
  `TermUebListRes3` / `HITUB` version 3 for responses.
- Keep Java-compatible frontend parameters: `my.country`, `my.blz`,
  `my.number`, `my.subnumber`, `startdate`, `enddate`, and `maxentries`.
- Render the account as national `KTV3` under `KTV`, matching the HBCI 300
  segment definition.
- Reuse the existing `GvrTermUebList` and `GvrTermUebListEntry` result
  structures instead of creating a separate classic scheduled-transfer result
  type.
- Extend the `GvrTermUebListEntry` extraction path so it reads classic response
  fields (`Other`, `BTG`, `key`, `addkey`, `usage`, `date`, and `id`) when no
  SEPA `sepapain` payload is present.
- Preserve XML-only `status` in raw `content.*` result data, but do not add it
  to the structured entry type in this slice because hbci4java's public result
  object does not expose it.
- Store `termueb_<id>` passport snapshots from classic list results using the
  same content-data snapshot helper as `TermUebSEPAList`.
- Verify account CRC for `my`, matching hbci4java's `checkAccountCRC("my")`.

## Consequences

This adds the classic scheduled-transfer list job without widening the public
API style. Reusing `GvrTermUebList` keeps the Java result-family shape intact,
while the parser now supports both SEPA payload-derived entries and classic
direct wire fields.
