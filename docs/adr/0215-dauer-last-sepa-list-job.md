# 0215 Port DauerLastSEPAList With PAIN.008 List Parsing

## Status

Accepted

## Context

`GVDauerLastSEPAList` in hbci4java retrieves recurring SEPA direct debit orders.
It is adjacent to `DauerLastSEPANew`, but it has a different risk profile because
its response parser reads PAIN direct-debit XML returned by the bank and maps it
into `GVRDauerLastList.Dauer`.

The pinned HBCI 300 protocol resource defines request segment
`DauerLastSEPAList1` with code `HKDDB` version 1 and response segment
`DauerLastSEPAListRes1` with code `HIDDB` version 1. The request accepts a KTV
account, one or more `sepadescr` values, optional `orderid`, optional
`maxentries`, and optional `offset`. hbci4java exposes `orderid` and
`maxentries` as frontend parameters and does not expose `offset`.

Unlike `DauerLastSEPANew`, hbci4java sets the list job default descriptor to
`SepaVersion.PAIN_008_001_02`. The Rust port currently only models
`PAIN_008_001_01`, so the list job needs the `PAIN_008_001_02` constant and
version metadata added.

The existing Rust `GvrDauerListEntry` already represents classic and SEPA
standing transfer list results, but it lacks several direct-debit fields that
hbci4java exposes for `GVRDauerLastList.Dauer`: `type`, `sequencetype`,
`creditorid`, `mandateid`, `manddateofsig`, and `endtoendid`.

## Decision

Port `DauerLastSEPAList` as an original-near list job:

- expose frontend job name `DauerLastSEPAList`;
- map constraints to `DauerLastSEPAList1`;
- use `PAIN_008_001_02` as the default `_sepadescriptor`;
- render request segment `HKDDB`;
- parse response segment `HIDDB` into `HbciJobResultData::DauerList`;
- extend `GvrDauerListEntry` with direct-debit fields present in
  hbci4java's `GVRDauerLastList.Dauer`;
- parse PAIN.008 list response XML with a small local-name-based parser covering
  the fields used by hbci4java result extraction;
- persist returned response snapshots under `dauer_<orderid>` like the existing
  `DauerSEPAList` implementation.

Keep `offset` unexposed in the frontend API for now because the upstream Java
constructor does not add a frontend constraint for it.

## Consequences

This brings the recurring direct debit list path into parity with the already
ported recurring transfer list path while preserving the descriptor difference
between submitting and listing direct debit orders.

The PAIN.008 parser is intentionally narrow. It is not a general SEPA parser; it
extracts only fields used by the hbci4java result object and can be expanded in a
later ADR when another job needs more PAIN.008 surface.
