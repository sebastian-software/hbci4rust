# ADR 0170: Account Info Job

## Status

Accepted

## Context

hbci4java exposes account master-data retrieval through the high-level job `AccInfo`
(`GVAccInfo`) and the result class `GVRAccInfo`. The request segment is `HKKIF`; in
HBCI-Plus/FinTS 3.0 the relevant segment shape is `AccInfo2`, with `AccInfoRes2` returning account
identity, account type, display names, opening date, interest rates, credit line, optional reference
account, delivery settings, free-form information, and address data.

The Rust port already has the `AccInfo` job name in the PinTAN registry, but rendering and typed
result extraction are not implemented. Existing account helper code can provide passport-account
fallbacks, but the generic account renderer includes SEPA fields that `AccInfo2.KTV` does not
support.

## Decision

Port `AccInfo` as an original-near first slice:

- keep the public job name `AccInfo`;
- use `AccInfo2`/`AccInfoRes2` (`HKKIF`/`HIKIF` version 2) for HBCI-Plus custom messages;
- add Java-near constraints for `my.country`, `my.blz`, `my.number`, `my.subnumber`, and `all`;
- render only the national account identity fields required by `AccInfo2.KTV`, not SEPA-only
  `iban`/`bic` fields;
- expose a typed Rust result variant mirroring `GVRAccInfo`'s observable fields with optional Rust
  values for fields that hbci4java represents with `null` or sentinel values;
- preserve the raw `AccInfoRes2` content in `HbciJobResult.result_data`, as other ported jobs do.

Older `AccInfo1`/`AccInfoRes1` support is deferred until fixtures or live-bank replays require it.
The first slice should stay close to the current FinTS 3.0/HBCI-Plus path and keep result parsing
focused on fields that the upstream result class exposes directly.

## Consequences

This adds a practical account-information job without jumping into payment generation. It also
introduces a reusable pattern for larger typed result classes: render the newest in-scope
HBCI-Plus segment first, keep `result_data`, and add typed result fields in a Java-recognizable
shape.

Open follow-up work:

- add `AccInfo1` parsing if older bank fixtures require it;
- port richer display formatting for `GVRAccInfo` if callers depend on exact text output;
- decide whether `all=J` should support account-less rendering once original behavior is traced
  against hbci4java.
