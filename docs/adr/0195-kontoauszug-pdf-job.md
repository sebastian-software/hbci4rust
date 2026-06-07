# ADR 0195: KontoauszugPdf Job

## Status

Accepted

## Context

`GVKontoauszugPdf` queries electronic account statements in PDF format. It is a
separate hbci4java job from `GVKontoauszug`, even though both return
`GVRKontoauszug`.

For HBCI 300 the protocol XML provides `KontoauszugPdf1` and
`KontoauszugPdf2`. Version 2 adds metadata such as time range, statement date,
statement year, statement number, account owner names, filename, and receipt.

The Java implementation always sets the result format to PDF. For the `booked`
payload it uses a pragmatic heuristic: if the payload starts with `%PDF-`, it is
treated as raw PDF bytes; otherwise it is decoded as Base64. The code ignores
the BPD `base64` flag for this decision.

## Decision

Port `KontoauszugPdf` as an original-near PDF statement query job.

- Use `KontoauszugPdf2` / `HKEKP` version 2 for HBCI 300 requests and
  `KontoauszugPdfRes2` / `HIEKP` version 2 for responses.
- Keep Java-compatible frontend parameters for account data plus `idx`,
  `year`, `maxentries`, and `offset`.
- Render account fields through `KTVInt`, accepting both SEPA fields and
  national fallback fields like hbci4java.
- Add `GvrKontoauszug`, `GvrKontoauszugEntry`, and `KontoauszugFormat` result
  types and map `KontoauszugPdf` responses to the PDF format.
- Decode PDF payloads with the same heuristic as Java: `%PDF-` means raw text
  bytes, otherwise Base64.
- Store receipt bytes as the decoded `Bin` content bytes. Do not interpret the
  receipt beyond preserving it.
- Defer general `Kontoauszug` / `HKEKA` support because that job includes
  selectable formats and MT940-specific umlaut decoding.

## Consequences

This adds a useful account-statement query while keeping the scope smaller than
the full electronic statement family. The result type can later be reused by
the general `Kontoauszug` job without changing the public result enum shape.
