# ADR 0153: PinTAN Signature Input Generation

## Status

Accepted

## Context

The existing PinTAN helpers deliberately accept deterministic values for
`seccheckref`, `secref`, date, and time. That keeps low-level replay tests stable
but leaves the runtime signer without the hbci4java-style inputs generated at
the `Sig.signIt(...)` boundary.

hbci4java fills those values while signing:

- `seccheckref` is generated from a random positive integer;
- the signature timestamp comes from the current clock;
- `secref` is the passport signature id;
- `AbstractPinTanPassport.incSigId()` is a no-op, so PinTAN keeps using the same
  signature id rather than persisting a counter.

The Rust handler should not bake random values directly into message tests. It
needs a small value object that can be generated in production and injected in
tests.

## Decision

Introduce `PinTanSignatureContext` in the manager signature module.

The context contains:

- `seccheckref`;
- `secref`;
- `timestamp_date`;
- `timestamp_time`.

Provide:

- deterministic constructors for tests and replay fixtures;
- a runtime constructor that generates an hbci4java-like positive random
  `seccheckref`, uses `secref = "1"` for PinTAN, and formats the current system
  timestamp as message-tree values `YYYY-MM-DD` / `HH:MM:SS`;
- a helper that turns the context plus `PinTanPassport` into `PinTanSigHead`.

The protocol datatype renderer still emits compact FinTS wire values
`YYYYMMDD` / `HHMMSS`. The context keeps the normalized message-tree shape so it
can pass through the same validation and rendering path as caller-provided
values.

Use UTC for the first Rust runtime timestamp formatter. This is not byte-for-byte
identical to hbci4java's local `Date` formatting in all time zones, but the
rendered field shape is protocol-compatible and keeps this slice dependency-free.
If live-bank evidence shows that local time matters, record and port that as a
later ADR.

## Consequences

The next handler signing slice can generate runtime signature metadata without
making message renderer tests nondeterministic.

Remaining work:

- wire `PinTanSignatureContext::generate()` into handler message signing;
- use the collected signature range when deciding one-step TAN requirements;
- revisit local-time formatting if live replay evidence requires it.

## Links

- `src/manager/signature.rs`
- `src/passport/pintan.rs`
- Upstream: `org.kapott.hbci.security.Sig.fillSigHead`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.incSigId`
