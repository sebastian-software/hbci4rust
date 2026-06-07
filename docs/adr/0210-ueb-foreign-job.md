# ADR 0210: UebForeign Job

## Status

Accepted

## Context

`GVUebForeign` submits a classic foreign transfer. It uses the lowlevel job name
`UebForeign`, returns hbci4java's generic `HBCIJobResultImpl`, and only performs
source account CRC validation in its `verifyConstraints()` override.

For HBCI 300 the protocol XML defines `UebForeign2` / `HKAOM` version 2. The
request segment contains a national `KTV3` source account, source account holder
name, optional national `KTV3` receiver account, optional receiver IBAN,
receiver bank name, receiver name, amount, cost-carrier code, and one optional
usage field. The HBCI 300 XML has `UebForeignPar2` / `HIAOMS` parameter data,
but no dedicated `UebForeignRes*` response segment.

Unlike domestic `Ueb`, hbci4java does not expose DTAUS transaction key data,
`name2`, or repeated usage lines for `UebForeign`. Receiver account fields and
receiver IBAN default to empty strings, while receiver name and receiver bank
name are required.

## Decision

Port `UebForeign` as an original-near classic foreign transfer job.

- Use `UebForeign2` / `HKAOM` version 2 for HBCI 300 requests.
- Do not parse a typed response or add a `HbciJobResultData` variant, because
  upstream has no specialized `GV_Result` class and the protocol XML has no
  response segment for this job.
- Keep Java-compatible frontend parameters:
  `src.country`, `src.blz`, `src.number`, `src.subnumber`, `src.name`,
  `dst.country`, `dst.blz`, `dst.number`, `dst.subnumber`, `dst.iban`,
  `dst.name`, `dst.kiname`, `btg.value`, `btg.curr`, `kostentraeger`, and
  `usage`.
- Keep Java defaults: `src.country=DE`, empty source subnumber, empty receiver
  country/BLZ/number/subnumber/IBAN, `kostentraeger=1`, and empty `usage`.
- Render the source account as national `KTV3` with the same source-account
  fallback shape as other classic payment renderers.
- Render the receiver `KTV3` only from supplied receiver values; do not apply a
  `DE` default to the receiver country because hbci4java's receiver country
  default is empty.
- Validate account CRC only for `src`, matching hbci4java.
- Defer BPD parameter interpretation for `caniban` and `countryinfo`; protocol
  XML validation still enforces the rendered field shapes and allowed
  `kostentraeger` values.

## Consequences

The Rust port can now render foreign-transfer orders for PinTAN dialogs and TAN
order-hash generation without introducing SEPA PAIN generation or result
parsing. Receiver account data remains deliberately permissive, matching the
legacy foreign-transfer segment shape.
