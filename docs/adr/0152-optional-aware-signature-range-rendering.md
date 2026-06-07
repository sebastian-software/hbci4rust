# ADR 0152: Optional-Aware Signature Range Rendering

## Status

Accepted

## Context

ADR 0151 introduced `collect_pintan_signature_range(...)` as the Rust-side
counterpart to hbci4java's `Sig.collectHashData(...)`. The collected range is
top-level message content from `SigHead` up to, but excluding, `SigTail`.

`CustomMsg` contains many optional top-level job placeholders. Those placeholders
carry protocol defaults in their child segment headers even when no job was
requested. A signature-range collector must therefore not decide renderability
by looking for any stored value, because default-only optional elements would be
misclassified as present.

The existing message renderer already has the original-near optional-element
rule: optional elements render only when they contain explicit or requested
message content. The signature range should use that same rule instead of
carrying a second heuristic.

## Decision

Expose a narrow `SyntaxElement` helper for rendering one top-level message child
with the same optional-aware rules used by full message rendering.

Use that helper in `collect_pintan_signature_range(...)` and skip top-level
children that render to `None`.

Keep `to_fints_string()` unchanged as the strict, direct renderer for callers
that already know an element must be renderable.

## Consequences

The PinTAN signature range now follows the same optional placeholder behavior as
normal outgoing messages. In `CustomMsg`, unused `GV` alternatives stay out of
the collected hash/signature range even though they contain generated or default
segment header values.

This reduces divergence between message rendering and signature preparation
before handler-level PinTAN signing is wired in.

## Links

- `docs/adr/0151-pintan-signature-range-collection.md`
- `src/manager/signature.rs`
- `src/protocol/message.rs`
- Upstream: `org.kapott.hbci.security.Sig.collectHashData`
