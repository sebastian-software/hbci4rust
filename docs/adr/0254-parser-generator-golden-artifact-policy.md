# ADR 0254: Parser And Generator Golden Artifact Policy

## Status

Accepted

## Context

The v1 release checklist requires risky parser/generator behavior to have Java
golden artifacts or explicit limitation entries.

The current Rust port already uses copied hbci4java fixtures and original-near
tests for protocol resources, FinTS wire/message behavior, CAMT, SWIFT/MT940,
bank-info parsing, and security-mechanism helpers. It also has PAIN parser and
generator tests that verify observable behavior, generated fields, sums, mixed
currency rejection, and parseability.

Not every risky behavior has a byte-for-byte Java output artifact. The most
important current example is PAIN XML generation: the Rust tests pin the v1
observable shape and roundtrip through the Rust parser, but they do not claim
that generated XML is byte-identical to hbci4java output.

## Decision

For v1, use three acceptable evidence categories:

- copied hbci4java fixtures checked into `tests/fixtures/hbci4java/`;
- Rust tests that pin original-near observable behavior against copied fixtures,
  known hbci4java test cases, or Java-compatible field semantics;
- explicit limitation entries for risky behavior that is intentionally not yet
  byte-for-byte Java-goldened.

Add `docs/reference/parser-generator-goldens.md` as the public release evidence
page for this policy. The page records:

- copied fixture inventory;
- current test coverage by parser/generator area;
- explicit v1 limitations for PAIN generator byte identity, uncopied upstream
  PAIN parse fixtures, partial MT942 behavior, and malformed bank responses;
- widening rules for adding new parser/generator behavior.

Mark the release checklist item for risky parser/generator behavior as covered
only through this documented mix of goldens and limitations. Keep the separate
malformed-bank-response replay/fixture item open until deterministic malformed
response coverage is broadened.

## Consequences

The port remains honest about original-near behavior: current tests cover many
observable Java-compatible behaviors, but v1 does not promise byte-identical
Java output for every generated XML document.

Future changes to parser/generator behavior must either add Java goldens or
extend the explicit limitation table before they are treated as release-ready.

The release checklist advances without pretending the offline parity surface is
fully exhausted.

## Links

- `docs/reference/parser-generator-goldens.md`
- `docs/architecture/release-checklist.md`
- `docs/architecture/v1-readiness.md`
- `tests/sepa.rs`
- `tests/swift.rs`
- `tests/structures.rs`
- `tests/protocol_resources.rs`
- `tests/protocol_wire.rs`
- `tests/protocol_message.rs`
- `tests/fixtures/hbci4java/`
- ADR 0007: Offline Test Strategy
- ADR 0246: V1 Release Checklist
