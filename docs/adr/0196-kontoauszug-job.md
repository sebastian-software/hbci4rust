# ADR 0196: Kontoauszug Job

## Status

Accepted

## Context

`GVKontoauszug` queries electronic account statements with selectable formats.
It shares hbci4java's `GVRKontoauszug` result type with `GVKontoauszugPdf`, but
uses the `HKEKA` / `HIEKA` segment family instead of the PDF-only `HKEKP` /
`HIEKP` family.

For HBCI 300 the protocol XML defines `Kontoauszug1` through `Kontoauszug5`.
Versions 1 to 3 use national `KTV3` account data, versions 4 and 5 use
`KTVInt`, and version 5 adds statement `date`, `year`, and `number` fields to
the response. The existing port has followed the highest HBCI-300 segment
version for similar query jobs, for example `Saldo7`, `KUmsZeit7`, and
`KontoauszugPdf2`.

The Java implementation decodes the `booked` payload differently from
`KontoauszugPdf`: it does not Base64-decode it. If the returned format is MT940
it first applies `Swift.decodeUmlauts`, then stores the resulting FinTS text
bytes. It also maps optional time range, statement metadata, information text,
account owner fields, and receipt bytes.

## Decision

Port `Kontoauszug` as the general original-near electronic statement query job.

- Use `Kontoauszug5` / `HKEKA` version 5 for HBCI 300 requests and
  `KontoauszugRes5` / `HIEKA` version 5 for responses.
- Reuse the existing `GvrKontoauszug`, `GvrKontoauszugEntry`, and
  `KontoauszugFormat` public result types introduced for `KontoauszugPdf`.
- Keep Java-compatible frontend parameters: `my.bic`, `my.iban`,
  national fallback account fields, `format`, `idx`, `year`, `maxentries`, and
  `offset`.
- Render account fields through `KTVInt` under `My`, accepting both SEPA and
  national fallback fields like hbci4java's segment-version-dependent
  constraints.
- Map response `format` codes through `KontoauszugFormat::from_code`.
- Preserve `booked` as bytes of the decoded FinTS text. For MT940 format,
  apply the existing `decode_umlauts` helper before storing bytes, matching
  hbci4java's `Swift.decodeUmlauts` call.
- Store receipt bytes as the decoded `Bin` content bytes and do not interpret
  them.

## Consequences

This completes the non-PDF side of the `GVRKontoauszug` result family for the
current HBCI-300-first tracer style. Older segment versions remain out of scope
until the port grows dynamic BPD-driven segment selection.
