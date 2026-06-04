# ADR 0009: Rustification Backlog

## Status

Accepted

## Context

The port should stay close to hbci4java until tests establish behavioral parity.
At the same time, Rust will eventually benefit from stronger types, smaller
modules, and less global state.

## Decision

Keep initial implementation package-near and Java-behavior-near.

Track later idiomatic Rust improvements in `docs/rustification/` rather than
silently applying them during the parity port.

## Consequences

The implementation can move quickly without losing later improvement ideas.
Rustification happens deliberately after tests protect behavior.

## Links

- `docs/rustification/README.md`
