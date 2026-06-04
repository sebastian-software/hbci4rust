# ADR 0004: Async Tokio Architecture

## Status

Accepted

## Context

hbci4java has synchronous APIs and special threaded callback flows. In Rust, a
direct thread model port would add complexity without matching modern async
application structure.

## Decision

Make the crate async-first and Tokio-based.

Do not create or own a runtime inside the library. Public operations return
futures and are awaited by the host application.

Callbacks use an async event/response trait. The Java `ThreadSyncer` and
`StringBuffer` callback result style are not ported.

Global runtime configuration mirrors `HBCIUtils`, but locks must not be held
across `.await`.

## Consequences

The Rust API intentionally diverges from Java threading while keeping protocol
states, callback reasons, job names, and property keys recognizable.

## Links

- `src/callback.rs`
- `src/comm/mod.rs`
- `src/manager/mod.rs`
