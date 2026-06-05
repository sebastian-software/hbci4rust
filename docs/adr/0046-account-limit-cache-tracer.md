# ADR 0046: Account Limit Cache Tracer

## Status

Accepted

## Context

hbci4java's public `Konto` structure exposes `limit`, and the matching
`Limit` structure contains:

- a limit type code (`E`, `T`, `W`, `M`, or `Z`);
- a monetary `Value`;
- optional day count for time limits.

The UPD `KInfo` segment provides this data in the optional `KLimit` group before
the repeated `AllowedGV` groups.

At the pinned upstream reference, `AbstractHBCIPassport#getAccounts()` reads
`KInfo.KLimit.*` into a local `Limit` instance but does not assign it back to
`entry.limit`. That looks like an upstream implementation gap rather than an
intentional public API absence, because `Konto.limit` and `Limit` remain public
structures.

The Rust port has started caching account metadata from UPD in Rust-native
passport data. Dropping `KLimit` would lose a parity-relevant part of the
account model and would make later account/job diagnostics less complete.

## Decision

Add a Rust-cased public `Limit` structure and expose it through
`Konto.limit: Option<Limit>`.

Use `Limit::TYPE_SINGLE`, `TYPE_DAILY`, `TYPE_WEEKLY`, `TYPE_MONTHLY`, and
`TYPE_TIME` string constants for the original one-character Java limit codes.

Import `KInfo.KLimit` from resolved `DialogInitRes.UPD` account values:

- `KLimit.limittype` into `Limit.limit_type`;
- optional `KLimit.BTG.value` and `KLimit.BTG.curr` into `Limit.value`;
- optional `KLimit.limitdays` into `Limit.days`.

Keep `Limit.value` optional because the protocol marks the `BTG` group as
optional, even though hbci4java's `Value(String, String)` constructor would not
handle a missing amount well.

When `PinTanPassport::fill_account_info(...)` fills an account from a cached
passport account, copy the cached limit only if the target account has none.

Record this as an intentional tiny divergence from the pinned hbci4java
implementation detail: Rust preserves the public account-limit intent instead
of reproducing the apparent missing `entry.limit = limit` assignment.

## Consequences

Rust-native passport storage can now persist account-level limits from UPD while
remaining backward compatible with existing JSON payloads through a
serde-defaulted `Option`.

Replay tests cover direct UPD import and handler-driven dialog init import.

The port still does not use limits for job validation, job filtering, or
user-facing warnings.

Remaining work:

- decide whether `Limit.value` should use a richer Java-like cent-integer money
  representation once `Value` is ported more fully;
- add time-limit fixtures using `Limit::TYPE_TIME` and `days`;
- apply account limits in job diagnostics only after the BPD/UPD job metadata
  model exists;
- compare live/replay UPD samples to confirm how often banks omit optional
  `BTG` values.

## Links

- `src/gv_result/mod.rs`
- `src/passport/pintan.rs`
- `src/manager/handler.rs`
- `tests/bootstrap.rs`
- `resources/protocol/hbci-300.xml`
- Upstream: `org.kapott.hbci.structures.Konto#limit`
- Upstream: `org.kapott.hbci.structures.Limit`
- Upstream: `org.kapott.hbci.passport.AbstractHBCIPassport#getAccounts`
