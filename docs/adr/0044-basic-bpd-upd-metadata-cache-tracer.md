# ADR 0044: Basic BPD And UPD Metadata Cache Tracer

## Status

Accepted

## Context

ADR 0043 introduced a narrow cache for BPD and UPD version numbers. hbci4java's
`AbstractHBCIPassport` exposes more basic values from the same parameter data:

- institution name from `BPA.kiname`;
- maximum jobs per message from `BPA.numgva`;
- maximum message size from `BPA.maxmsgsize`;
- supported languages from `BPA.SuppLangs.lang`;
- supported HBCI versions from `BPA.SuppVersions.version`;
- user parameter usage from `UPA.usage`;
- user display name from `UPA.username`.

The Rust port still does not have full BPD/UPD property bags. Adding the full
property model now would be larger than the current tracer layer needs.

## Decision

Add Rust-native optional fields for the basic BPD/UPD metadata listed above to
`PinTanPassportData`.

Import them from flat `DialogInitRes` values via
`PinTanPassport::update_parameter_data_from_values(...)`.

Keep `update_parameter_versions_from_values(...)` as the smaller helper and let
the new broader method call it.

Expose small accessors on `PinTanPassport`, including `only_bpd_gvs()` for the
hbci4java `UPA.usage == 0` behavior.

Do not introduce full Java-like BPD/UPD `Properties` storage in this slice.

## Consequences

The PinTAN passport can now carry the simplest bank and user parameter metadata
through Rust-native storage and replay tests.

The handler still uses only BPD/UPD versions for outgoing `DialogInit`; the new
metadata is available for later job filtering, sync decisions, and docs.

Remaining work:

- port full BPD/UPD property bags once more parameter segments are needed;
- import communication parameters, security methods, TAN media metadata, and
  job parameter groups;
- decide how to migrate from this Rust-native subset to richer parameter
  storage without losing existing passport files;
- decide whether numeric fields should preserve original strings in addition to
  parsed integers.

## Links

- `src/passport/pintan.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `resources/protocol/hbci-300.xml`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#getInstName`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#getMaxGVperMsg`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#getMaxMsgSizeKB`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#getSuppLangs`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#getSuppVersions`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#onlyBPDGVs`
