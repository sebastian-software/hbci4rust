# ADR 0051: Konto Display Tracer

## Status

Accepted

## Context

hbci4java's `Konto.toString()` renders account details in a fixed order:

- account type;
- account holder name lines;
- account number and optional subnumber;
- BLZ plus bank name from `HBCIUtils.getNameForBLZ(...)`;
- BIC;
- IBAN;
- country;
- currency.

The Rust port had no equivalent display implementation yet. Result types can
therefore not use the familiar account rendering from hbci4java.

The port also does not yet include hbci4java's bank-info data table or
`HBCIUtils.getNameForBLZ(...)`.

## Decision

Implement `Display` for `Konto` as the Rust equivalent of hbci4java
`Konto.toString()`.

Preserve the original field order and spacing.

Render the BLZ bank-name slot as an empty placeholder for now:
`BLZ <blz> ()`.

Do not add a bank-info table, `HBCIUtils` facade, or BLZ-name lookup in this
slice.

Use standard Rust `to_string()` via `Display` instead of adding a Java-style
method name.

## Consequences

`Konto` now has a stable human-readable representation for debug output and
future result renderers.

The representation is original-near in ordering and shape, but incomplete where
hbci4java would consult bank metadata.

Tests pin the current no-bank-info rendering so a future bank-info tracer can
change it deliberately.

Remaining work:

- port `BankInfo`, `blz.properties`, and `HBCIUtils.getNameForBLZ(...)`;
- decide whether `Konto::default()` should mirror Java's `new Konto()` currency
  default before testing default-account display;
- update result `Display`/summary helpers to use `Konto` display once those
  hbci4java result renderers are ported.

## Links

- `src/gv_result/mod.rs`
- `tests/bootstrap.rs`
- Upstream: `org.kapott.hbci.structures.Konto#toString`
- Upstream: `org.kapott.hbci.manager.HBCIUtils#getNameForBLZ`
- Upstream: `org.kapott.hbci.manager.BankInfo`
