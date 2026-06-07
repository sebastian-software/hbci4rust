# ADR 0135: Orderhash Binary Rendering

## Status

Accepted

## Context

hbci4java calculates the HKTAN process-1 order hash from the rendered business
transaction segment:

1. render the task segment with hbci4java's FinTS syntax;
2. encode the rendered segment with `Comm.ENCODING` (`ISO-8859-1`);
3. hash it using the bank-selected `orderhashmode`;
4. set the HKTAN `orderhash` parameter, where `GVTAN2Step.setParam(...)`
   prefixes `B`;
5. render that value as a FinTS `Bin` element.

For PinTAN BPD, hbci4java maps `orderhashmode=1` to `RIPEMD160` and
`orderhashmode=2` to `SHA-1`.

The current Rust message renderer mostly returns `String`, which is sufficient
for ASCII fixtures but not for arbitrary digest bytes. A SHA-1 or RIPEMD160
digest can contain any byte value, while FinTS `Bin` payloads are length
prefixed byte arrays.

## Decision

Add an additive binary-safe render path:

- keep `to_fints_string()` for existing string golden tests;
- add `to_fints_bytes()` to `HbciMessage` and `SyntaxElement`;
- make `HbciHandler` send `to_fints_bytes()` request bodies;
- render `Bin` byte payloads by interpreting `B...` payload characters as
  ISO-8859-1 bytes;
- keep non-`Bin` elements on the existing string renderer for now.

Add an `OrderHashMode` helper:

- `OrderHashMode::from_code("1")` -> RIPEMD160;
- `OrderHashMode::from_code("2")` -> SHA-1;
- unsupported codes return an error, matching hbci4java's strict mode handling;
- `hash_segment(...)` returns the raw digest bytes represented as
  ISO-8859-1 characters;
- `hash_segment_bin(...)` returns the HKTAN-ready `B`-prefixed value.

Use RustCrypto `sha1` and `ripemd` crates for the digest implementations
instead of reimplementing cryptographic primitives in this port.

## Consequences

The Rust port can now represent and send true binary HKTAN order hashes without
distorting bytes through UTF-8 string encoding.

Remaining work:

- derive `orderhashmode` from BPD/SecMech automatically;
- render the originating task segment for hashing inside the PinTAN dialog
  flow;
- broaden byte rendering for all non-ASCII FinTS text fields if live-bank
  tests expose an ISO-8859-1 mismatch outside `Bin`.

## Links

- `src/manager/orderhash.rs`
- `src/protocol/datatype.rs`
- `src/protocol/message.rs`
- `src/manager/handler.rs`
- Upstream: `org.kapott.hbci.tools.CryptUtils.hash`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.getOrderHashMode`
