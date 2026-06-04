# ADR 0007: Offline Test Strategy

## Status

Accepted

## Context

The upstream test suite includes offline fixtures as well as optional tests that
need live bank access or special hardware. v1 excludes hardware and must not
depend on real credentials in CI.

## Decision

CI runs offline only: `cargo fmt`, `cargo clippy --all-targets`, and
`cargo test`.

Port in-scope offline hbci4java tests and add Java-generated golden artifacts
for risky behavior: SEPA XML, CAMT parse summaries, MT940 parse summaries,
BPD/message parsing, and PinTAN dialog replays.

Live-bank tests, when added, must be ignored by default and enabled only through
explicit environment variables.

## Consequences

The port can be developed safely and repeatably. Compatibility with real banks is
validated manually or in explicitly provisioned environments.

## Links

- `scripts/generate-goldens.sh`
- `tests/`
