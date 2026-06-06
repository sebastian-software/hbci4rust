# ADR 0052: Bank Info Parser Tracer

## Status

Accepted

## Context

hbci4java ships a `blz.properties` file and parses each value into
`org.kapott.hbci.manager.BankInfo`.

The value columns are pipe-separated:

1. bank name;
2. location;
3. BIC;
4. account checksum method;
5. RDH address;
6. PinTAN address;
7. RDH HBCI version;
8. PinTAN HBCI version.

`HBCIUtils.refreshBLZList(...)` then sets the BLZ from the property key and
stores all parsed records in a global map. `HBCIUtils.getNameForBLZ(...)` reads
that map and returns an empty string for unknown bank codes.

The Rust port now has `Konto` display, but it still renders the bank-name slot
as an empty placeholder because no bank-info table exists yet.

## Decision

Add a minimal `BankInfo` parser tracer under the Rust `manager` module.

Expose:

- `BankInfo`;
- `HbciVersion`.

Port the original HBCI version IDs:

- `201`;
- `210`;
- `220`;
- `plus`;
- `300`;
- `400`.

Use `BankInfo::parse_value(...)` for hbci4java `BankInfo.parse(...)` value
parsing, and `BankInfo::parse_property(blz, value)` for the
`refreshBLZList(...)` pattern that sets the BLZ from the property key.

Mimic Java `String.split("\\|")` with its default behavior of dropping
trailing empty columns while preserving intermediate empty columns.

Implement `Display` for `BankInfo` and `HbciVersion` as the Rust equivalents of
the upstream `toString()` methods.

Do not vendor `blz.properties` in this slice. Do not add a global bank-info map,
`refresh_blz_list`, `get_bank_info`, or `get_name_for_blz` yet. Do not wire
`Konto::Display` to bank lookup yet.

## Consequences

The port now has the typed parser needed for later BLZ resource loading and
bank-name display.

Tests pin the column mapping, HBCI version IDs, Java split behavior for trailing
empty columns, and empty-object display shape.

The visible `Konto` display output remains unchanged until the bank-info
repository is introduced deliberately.

Remaining work:

- decide whether to vendor the full pinned `blz.properties` file or generate a
  compact fixture for tests;
- port `HBCIUtils.refreshBLZList(...)`, `getBankInfo(...)`, and
  `getNameForBLZ(...)`;
- wire `Konto::Display` to bank-name lookup once there is an explicit lookup
  source;
- use `checksum_method` as input to a later account-CRC tracer.

## Links

- `src/manager/bank_info.rs`
- `src/manager/mod.rs`
- `src/lib.rs`
- `tests/bank_info.rs`
- Upstream: `org.kapott.hbci.manager.BankInfo`
- Upstream: `org.kapott.hbci.manager.HBCIVersion`
- Upstream: `org.kapott.hbci.manager.HBCIUtils#refreshBLZList`
- Upstream: `org.kapott.hbci.manager.HBCIUtils#getNameForBLZ`
- Upstream resource: `src/main/resources/blz.properties`
