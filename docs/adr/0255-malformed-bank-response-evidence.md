# ADR 0255: Malformed Bank Response Evidence

## Status

Accepted

## Context

The v1 release checklist requires malformed-bank-response behavior added before
v1 to have deterministic replay or fixture coverage.

The current port already has several deterministic offline evidence layers:

- FinTS wire and message tests reject malformed syntax, invalid binary blocks,
  invalid segment sequences, unsupported defaults, invalid valids, unknown
  segment headers, and repeated message elements beyond protocol maxima.
- Handler replay tests reject mismatched response references and dialog IDs
  during init, custom-message execution, and close.
- PinTAN/SCA replay tests cover failed process-2 submission state, transport
  retry state, missing order references, decoupled polling bounds, and invalid
  QR payload fallback behavior.
- Status tests cover bank-side return-code inspection, including invalid PIN
  detection and segment/global status behavior.

These tests are meaningful release evidence, but they are not an exhaustive
claim that arbitrary malformed live-bank responses are handled.

## Decision

Add `docs/reference/malformed-bank-responses.md` as the public evidence page for
malformed and unexpected bank-response behavior.

For v1, a malformed-response behavior is release-acceptable only when it is
covered by one of these named evidence types:

- protocol fixture or unit coverage in `tests/protocol_wire.rs` or
  `tests/protocol_message.rs`;
- deterministic handler replay coverage through `ReplayCommClient`;
- status or security-mechanism tests that pin the observable API behavior;
- an explicit limitation entry on the malformed-response reference page.

Mark the release checklist malformed-response item as covered for the current
v1 surface only through that documented evidence and limitation set.

Do not treat this as a blanket robustness promise. Any new malformed
bank-response behavior added before v1 must add deterministic replay or fixture
coverage, or extend the limitation entry before release acceptance depends on
it.

## Consequences

The checklist can advance because the current malformed-response surface is
named, test-backed, and bounded.

Future live-bank observations remain outside CI. They must be anonymized and
converted into deterministic replay fixtures, copied fixtures, or explicit
limitations before they influence v1 acceptance.

The port stays original-near and honest: it records what is tested without
pretending to have exhaustive real-bank coverage.

## Links

- `docs/reference/malformed-bank-responses.md`
- `docs/reference/parser-generator-goldens.md`
- `docs/reference/error-reporting.md`
- `docs/architecture/release-checklist.md`
- `docs/architecture/v1-readiness.md`
- `tests/protocol_wire.rs`
- `tests/protocol_message.rs`
- `tests/bootstrap.rs`
- `tests/status.rs`
- `tests/secmech.rs`
- `tests/runtime_callbacks.rs`
- ADR 0007: Offline Test Strategy
- ADR 0246: V1 Release Checklist
- ADR 0254: Parser And Generator Golden Artifact Policy
