# ADR 0043: BPD And UPD Version Cache Tracer

## Status

Accepted

## Context

`DialogInit` currently sent `ProcPrep.BPD = 0` and `ProcPrep.UPD = 0` for every
request. That is acceptable for the earliest replay tracer, but it is not how
hbci4java behaves once bank and user parameter data exists.

hbci4java stores BPD and UPD as property maps in the passport. Its
`AbstractHBCIPassport#getBPDVersion()` returns `BPA.version` or `0`, and
`getUPDVersion()` returns `UPA.version` or `0`. `AbstractRawHBCIDialogInit`
copies those values into `ProcPrep.BPD` and `ProcPrep.UPD`.

The Rust port does not yet have complete BPD/UPD property bags.

## Decision

Add a narrow Rust-native cache for only the BPD and UPD version numbers:

- `PinTanPassportData.bpd_version`;
- `PinTanPassportData.upd_version`.

Both fields are optional and serde-defaulted for compatibility with existing
passport JSON payloads. Accessors return `0` when the value is absent, matching
hbci4java's version fallback.

After parsing `DialogInitRes`, update these fields from:

- `DialogInitRes.BPD.BPA.version`;
- `DialogInitRes.UPD.UPA.version`.

When rendering `DialogInit`, set:

- `DialogInit.ProcPrep.BPD = passport.bpd_version()`;
- `DialogInit.ProcPrep.UPD = passport.upd_version()`.

Do not introduce full BPD/UPD property storage in this slice.

## Consequences

The next `DialogInit` can now advertise the cached parameter versions instead
of always forcing `0/0`.

Replay tests cover both direct protocol-value import and handler request
rendering from cached versions.

Remaining work:

- port full BPD storage for bank parameters and supported jobs;
- port full UPD storage for usage rules, TAN media helper keys, and protected
  user-side metadata;
- decide migration rules for richer passport data once full property bags are
  introduced;
- use BPD/UPD versions to drive sync decisions instead of only rendering them.

## Links

- `src/passport/pintan.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `resources/protocol/hbci-300.xml`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#getBPDVersion`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#getUPDVersion`
- Upstream: `org.kapott.hbci.dialog.AbstractRawHBCIDialogInit`
