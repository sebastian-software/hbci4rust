# ADR 0045: Account Allowed GV Cache Tracer

## Status

Accepted

## Context

hbci4java exposes account-specific allowed job codes through
`Konto.allowedGVs`. `AbstractHBCIPassport#getAccounts()` builds that list from
UPD account parameter data by iterating `KInfo.AllowedGV*.code` until the first
missing code.

The upstream tests also capture the Java property path convention for repeated
allowed job groups, including `KInfo.AllowedGV_2.code`.

The Rust port already imports basic UPD account data into
`PinTanPassportData.accounts`, but it did not preserve account-level allowed GV
codes. Without that cache, later job filtering and diagnostics would have to
re-parse raw UPD values or skip a parity-relevant part of `Konto`.

## Decision

Add `Konto.allowed_gvs: Vec<String>` as a serde-defaulted field.

Import account allowed job codes from repeated UPD `AllowedGV` groups:

- `DialogInitRes.UPD.KInfo.AllowedGV.code`;
- `DialogInitRes.UPD.KInfo.AllowedGV_2.code`;
- and subsequent counted group paths.

Use the existing resolved wire-message path shape, where repeated data element
groups receive the suffix on the group name, not on the leaf data element.

When `PinTanPassport::fill_account_info(...)` fills missing account data from a
cached passport account, copy non-empty `allowed_gvs` as well.

Do not model the rest of `AllowedGV` yet:

- required signatures;
- limit type;
- limit value and currency;
- limit days.

Do not enforce `allowed_gvs` in `SaldoReq` or `SaldoReqAll` in this slice. The
list is cached for later job selection, filtering, and user-facing diagnostics.

## Consequences

Rust-native passport storage now preserves another parity-relevant piece of UPD
account metadata while remaining backward compatible with existing JSON
payloads.

Replay tests cover both direct UPD import and handler-driven dialog init import.

The test fixtures include a minimal valid `KLimit` before `AllowedGV` because
the original `KInfo6` segment places optional `KLimit` before the repeated
`AllowedGV` groups.

Remaining work:

- model `AllowedGV` signatures and limits when job filtering needs them;
- decide whether `Konto` should expose richer allowed-job metadata or keep only
  the Java-compatible code list;
- apply `allowed_gvs` during job creation/execution once the BPD/UPD job
  registry is available;
- preserve or expose raw UPD property bags if later tracers need exact Java
  property semantics.

## Links

- `src/gv_result/mod.rs`
- `src/passport/pintan.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `resources/protocol/hbci-300.xml`
- Upstream: `org.kapott.hbci.structures.Konto#allowedGVs`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#getAccounts`
- Upstream test: `org.kapott.hbci4java.bpd.AllowedGVTest`
