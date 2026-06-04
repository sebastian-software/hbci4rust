# ADR 0003: V1 PinTAN Scope

## Status

Accepted

## Context

The upstream project contains many historical security media and native
components: Chipcard, PCSC, CTAPI, DDV, RDH, RAH, and RSA key-file paths.

Current consumer-bank usage is centered on FinTS PinTAN / HBCI-Plus over HTTPS
with TAN or app-based SCA. Chipcard and key-file media are legacy or niche and
would dominate implementation risk.

## Decision

v1 supports FinTS PinTAN / HBCI-Plus only.

Exclude Chipcard, PCSC, CTAPI, DDV, RDH, RAH, RSA key-file live support, and
Java passport import from v1.

## Consequences

The first useful port can focus on modern consumer-bank behavior. Historical
upstream documentation remains archived as context, but these paths are not
ported unless a later ADR reopens them.

## Links

- Rustification backlog: `docs/rustification/README.md`
