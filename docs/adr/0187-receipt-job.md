# ADR 0187: Receipt Job

## Status

Accepted

## Context

`GVReceipt` is hbci4java's high-level job for sending a receipt to the bank.
The pinned protocol resources define `Receipt1` / `HKQTG` in both
`hbci-300.xml` and `hbci-plus.xml`. The request segment contains only the
signature/user segment header and an optional binary `receipt` data element.

The original Java job has a single public constraint:

- `receipt` -> `Receipt1.receipt`, defaulting to an empty value.

`GVReceipt#setParam("receipt", value)` marks the frontend value as a binary
FinTS value by prefixing it with `B` before delegating to `HBCIJobImpl`. It
uses the base `HBCIJobResultImpl` result and therefore has no dedicated
structured response parser.

## Decision

Port `Receipt` as a small PinTAN job slice:

- expose the original-near `receipt` constraint for `Receipt1`;
- render `Receipt1` as `HKQTG` inside signed custom messages;
- treat the public Rust `receipt` parameter as the raw receipt payload and
  prefix it with `B` at render time to match hbci4java's binary marker
  behavior;
- keep successful responses represented by the generic `HbciJobResult` status
  and `result_data` map, without adding a dedicated result enum variant.

Do not port electronic account statement receipt generation, automatic
receipt follow-up handling, or `Kontoauszug` / `KontoauszugPdf` in this slice.

## Consequences

The Rust port can send replay-tested receipt acknowledgements through the
existing signed PinTAN custom-message path and reuse the current binary data
element renderer.

The renderer-level `B` prefix keeps the public API ergonomic for the original
frontend parameter while still allowing the protocol layer to produce the
`@len@payload` FinTS wire shape.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVReceipt.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `target/reference/hbci4java/src/main/resources/hbci-plus.xml`
- `docs/adr/0156-signed-pintan-custom-message.md`
