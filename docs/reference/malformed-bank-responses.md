# Malformed Bank Response Evidence

Snapshot date: 2026-06-07.

This page records deterministic v1 evidence for malformed or unexpected
bank-response behavior. It supports ADR 0255 and the release checklist item that
requires malformed-bank-response behavior added before v1 to have replay or
fixture coverage.

It is not a claim that hbci4rust handles every possible malformed live-bank
response. The v1 claim is limited to the named tests and explicit limitations
below.

## Evidence Matrix

| Area | Evidence | Current v1 claim | Limitation |
| --- | --- | --- | --- |
| Wire syntax | `tests/protocol_wire.rs`; `tests/protocol_message.rs` | Malformed FinTS messages, unterminated quoted values, malformed binary blocks, invalid binary lengths, unknown segment headers, incomplete segment headers, repeated message elements beyond protocol maxima, and invalid signature-range input are rejected deterministically. | This is parser-level coverage, not a replay of every real-bank response shape. |
| Protocol metadata validation | `tests/protocol_wire.rs`; `tests/protocol_message.rs` | Incoming values that conflict with protocol defaults or valids, wrong message definitions, invalid segment sequences, invalid country datatypes, and invalid segment sequence numbers are rejected or accepted according to the current original-near metadata rules. | The port does not remodel the FinTS spec beyond the copied hbci4java XML resources. |
| Response identity | `tests/bootstrap.rs` | Handler init, custom-message execution, and dialog close reject mismatched response references or dialog IDs through deterministic `ReplayCommClient` responses. | More bank-specific reference quirks need replay fixtures before release acceptance depends on them. |
| SCA and TAN state resilience | `tests/bootstrap.rs` | Missing process-2 or decoupled order references, decoupled polling beyond BPD maximums, failed process-2 submission state, and process-2 transport retry state are covered with deterministic replays. | New SCA processes or bank-specific state transitions discovered during live smoke testing remain outside v1 until replayed or documented. |
| Bank-side status and segment errors | `tests/bootstrap.rs`; `tests/status.rs`; `docs/reference/error-reporting.md` | Segment and global return values remain inspectable through `HbciMsgStatus`, `HbciExecStatus`, job result status, error strings, and known invalid-PIN helpers. Dialog close accepts a segment error when the global status is okay, matching the current original-near behavior. | Bank return text and params are not normalized into a closed enum; callers must inspect returned status data. |
| TAN media payload surprises | `tests/bootstrap.rs`; `tests/secmech.rs` | QR/photoTAN payloads, matrix-code helpers, challenge-info fixtures, and invalid QR payload fallback to the generic TAN callback are covered. | New TAN media payload variants from live banks need fixture or replay coverage before v1 acceptance depends on them. |
| Transport replay behavior | `tests/bootstrap.rs`; `tests/runtime_callbacks.rs`; `src/comm/replay.rs` | The replay client is used for offline handler paths, connection callbacks, and transport-error state preservation without live bank credentials. | Network behavior in the default HTTPS client is not used as CI acceptance evidence. |

## Current Release Boundary

For v1, malformed-bank-response behavior is considered covered only when one of
these is true:

- the behavior is named in the matrix above and backed by the referenced tests;
- the behavior is covered by a copied fixture under `tests/fixtures/hbci4java/`;
- the behavior has a deterministic `ReplayCommClient` test;
- the behavior is listed as an explicit limitation on this page or a linked
  reference page.

Any new malformed-response behavior added before v1 must update this page and
add deterministic replay or fixture coverage before it can be treated as
release-ready.

## Live Observation Rule

Manual live-bank smoke observations must not include credentials, PINs, TANs,
or full personal account data in docs or fixtures.

If a live run exposes malformed or unexpected behavior that should influence
v1 acceptance, convert it into one of these artifacts:

- an anonymized deterministic replay test;
- a copied or generated offline fixture that contains no secrets;
- an explicit limitation entry with the affected public behavior.

The regular CI and v1 acceptance gates remain offline-only.

## Recheck Commands

```sh
cargo test --test protocol_wire
cargo test --test protocol_message
cargo test --test bootstrap
cargo test --test status
cargo test --test secmech
cargo test --test runtime_callbacks
```

The full release-candidate gate still runs `cargo test` and records the complete
output summary after the last release-candidate commit.

## References

- `docs/adr/0007-offline-test-strategy.md`
- `docs/adr/0246-v1-release-checklist.md`
- `docs/adr/0250-error-reporting-review.md`
- `docs/adr/0254-parser-generator-golden-artifact-policy.md`
- `docs/adr/0255-malformed-bank-response-evidence.md`
- `docs/architecture/release-checklist.md`
- `docs/architecture/v1-readiness.md`
- `docs/reference/error-reporting.md`
- `docs/reference/parser-generator-goldens.md`
- `docs/reference/live-bank-tests.md`
