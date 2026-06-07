# ADR 0243: Decoupled PinTAN Callback Code Mapping

## Status

Accepted

## Context

ADR 0242 added the first Rust building block for decoupled PinTAN status
polling: a `TAN2Step`/`HKTAN` status request with `process=S`. The next runtime
slice needs callback reasons that can tell applications when decoupled approval
is expected or still pending.

hbci4java exposes two dedicated PinTAN callback constants:

- `HBCICallback.NEED_PT_DECOUPLED = 35`;
- `HBCICallback.NEED_PT_DECOUPLED_RETRY = 36`.

The Rust callback enum currently preserves unknown codes, but these two reasons
are no longer merely unknown: they are in v1 scope because decoupled PinTAN
polling is now being ported.

## Decision

Add Rust-cased callback reasons for the two hbci4java decoupled PinTAN codes:

- `CallbackReason::NeedPtDecoupled`;
- `CallbackReason::NeedPtDecoupledRetry`.

Map them to and from the original hbci4java constants `35` and `36` through
`original_code()` and `from_original_code(...)`.

Keep PhotoTAN and QR-TAN callback constants out of this slice. ADR 0245 records
their dedicated callback emission once that runtime decision is made.

## Consequences

The public callback surface can now represent decoupled approval and retry
events without falling back to `Unknown(i32)`. ADR 0244 uses these reasons for
the `3956` refresh loop while keeping this mapping change small and
original-near.

Tests must pin the numeric mappings so later callback work does not drift from
hbci4java constants.

## Links

- ADR 0070: Callback Original Code Mapping Tracer
- ADR 0143: SCA TAN Callback Helper
- ADR 0242: Decoupled PinTAN Status Request Helper
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/callback/HBCICallback.java`
