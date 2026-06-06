# ADR 0107: KUms MT940 Multitag Mapping

## Status

Accepted

## Context

ADR 0106 ported MT940 `:61:` transaction-line parsing into `GvrKUmsLine`, but
left the following `:86:` transaction details empty. hbci4java maps the `:86:`
value with `Swift.packMulti(...)` and `Swift.getMultiTagValue(...)`.

The upstream mapping fills bank transaction code, text, primanota, usage lines,
counter-account data, additional key, SEPA hints, and an opaque `additional`
field for `gvcode == 999`.

Important original behavior:

- the first three characters of `:86:` become `gvcode`;
- the remainder is packed with `Swift.packMulti(...)`;
- `gvcode == 999` bypasses structured extraction and stores the packed remainder
  in `additional`;
- `gvcode` values starting with `1` mark SEPA transactions;
- counter-account BIC/BLZ values are trimmed at the first space;
- if any counter-account field exists, hbci4java fills missing `blz`, `number`,
  and `name` with empty strings before assigning `other`.

## Decision

Port `:86:` multitag mapping into `GvrKUmsLine`.

This slice fills:

- `gvcode`;
- `is_sepa`;
- `text`;
- `primanota`;
- `usage` from tags `20` through `29` and `60` through `63`;
- `other` from tags `30` through `33`;
- `addkey`;
- `additional` for `gvcode == 999`.

Keep the implementation original-near and use the already ported `swift`
helpers. Do not normalize labels, split SEPA remittance fields further, or
change the public structure names.

Do not integrate structured `GvrKUms` into handler execution in this slice.

## Consequences

`GvrKUms::get_flat_data()` now exposes the common MT940 transaction details
needed by many account-turnover callers.

Remaining work:

- port final balance correction after closing balance parsing;
- integrate `GvrKUms` into `KUmsAll`/`KUmsNew` handler results;
- port upstream `TestMT940Parse` as fixture-based tests once handler/result
  integration is present;
- decide parser error reporting for malformed `:86:` values.

## Links

- `src/gv_result/mod.rs`
- `tests/structures.rs`
- `docs/adr/0106-kums-mt940-line-parser.md`
- Upstream: `org.kapott.hbci.GV_Result.GVRKUms#parseMT94x`
