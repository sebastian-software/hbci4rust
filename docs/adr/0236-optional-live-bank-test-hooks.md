# 0236 Add Optional Live Bank Test Hooks

## Status

Accepted

## Context

The v1 acceptance criteria and CI remain offline-only. Most parity work is
covered by replay fixtures, generated goldens, and protocol-level tests.

At the same time, Phase 4 of the porting plan calls for optional live-bank test
hooks. Those hooks are useful for manually validating the PinTAN dialog boundary
against a real endpoint, but they must not make the repository depend on real
bank credentials, real TAN devices, or network availability.

## Decision

Add ignored, environment-gated live tests:

- live tests are not run by `cargo test`;
- they require both `cargo test -- --ignored` and
  `HBCI4RUST_LIVE_ENABLE=1`;
- credentials and endpoints are read only from environment variables;
- no credential values are logged;
- the initial hook exercises only dialog init and close, not payment or account
  jobs;
- missing environment values fail with explicit variable names only after the
  live gate is enabled.

Keep these hooks out of CI unless a future ADR adds a dedicated secret-managed
live test environment.

## Consequences

Developers can manually test the current PinTAN transport/dialog shell against a
real bank without weakening offline CI.

The live hook does not prove all bank behavior. Replay fixtures and protocol
tests remain the source of deterministic acceptance evidence.

If later live testing needs transactions, TAN challenge inspection, or bank-
specific assertions, add them as separate ignored tests and record the decision
in a new ADR.

## References

- `docs/adr/0007-offline-test-strategy.md`
- `docs/adr/0154-signed-pintan-dialog-init.md`
- `docs/adr/0155-signed-pintan-dialog-end.md`
- `src/comm/mod.rs`
- `src/manager/handler.rs`
