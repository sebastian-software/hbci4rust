# ADR 0056: Structure Display Tracer

## Status

Accepted

## Context

hbci4java's public structures include human-readable `toString()` methods.

The Rust port already carries the current tracer equivalents of:

- `Value`;
- `Saldo`;
- `Limit`.

They were serializable result structures, but they had no display output yet.
That made them less useful for original-near result summaries and future
`GVR*` display ports.

The upstream behavior is:

- `Value.toString()` renders `<amount> <currency>` with a dot decimal separator
  and two decimal places;
- `Saldo.toString()` renders `<timestamp> <value>`;
- `Limit.toString()` renders a German limit label followed by `: <value>`.

The Rust `Value` is still a string-backed tracer, not Java's cent-integer money
type.

## Decision

Implement `Display` for `Value`, `Saldo`, and `Limit`.

For `Value`, format simple decimal strings with two decimal places and append
the currency, using `null` when `curr` is absent. Remove spaces from the amount
before formatting, matching hbci4java's `Value(String, String)` constructor.

Keep unparseable or higher-precision tracer strings unchanged rather than
rounding or failing in `Display`.

For `Saldo`, render the stored date and time strings directly, followed by the
displayed value. Do not introduce localized date-time formatting in this slice.

For `Limit`, port the original German labels:

- `Einzellimit`;
- `Tageslimit`;
- `Wochenlimit`;
- `Monatslimit`;
- `Zeitliches Limit (<days> Tage)`.

Use `null` for an absent Rust `Limit.value`, because the Rust structure keeps
the protocol's optional `BTG` group instead of assuming Java's non-null field.

Do not replace the string-backed money representation with a cent-integer or
decimal type in this slice.

## Consequences

The existing result structures now have original-near display output without
changing their storage shape.

The implementation is intentionally a tracer. It is suitable for the current
offline Saldo and UPD limit work, but it does not prove full Java money or
locale parity.

Remaining work:

- decide whether `Value` should become a Java-like cent-integer type;
- decide whether localized date-time formatting belongs in the library API;
- add `Display` implementations for `GVRSaldoReq` and other result structures
  as their ports become richer.

## Links

- `src/gv_result/mod.rs`
- `tests/structures.rs`
- Upstream: `org.kapott.hbci.structures.Value#toString`
- Upstream: `org.kapott.hbci.structures.Saldo#toString`
- Upstream: `org.kapott.hbci.structures.Limit#toString`
