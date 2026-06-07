# ADR 0192: Verification Of Payee Authorization Job

## Status

Accepted

## Context

`GVVoPAuth` is hbci4java's high-level job for authorizing a previously received
Verification of Payee result. The Java job name and lowlevel segment name are
both `VoPAuth`.

The pinned FinTS 3.0 resource defines `VoPAuth1` / `HKVPA`. The request segment
contains one required binary data element:

- `vopid`, the VoP id returned by `VoPCheckRes1` / `HIVPP`.

`GVVoPAuth#setParam("vopid", value)` prefixes the value with `B` before storing
it, matching hbci4java's binary data-element convention. There is no dedicated
`VoPAuth` response segment in the resource; success and failure are represented
through ordinary return values.

`GVVoP` itself is much larger: it parses `VoPCheckRes1`, handles polling,
callbacks, persistent passport data, and queues `VoPAuth` together with the
original task. That orchestration is not part of this compact authorization
slice.

## Decision

Port `VoPAuth` as a small original-near PinTAN job slice:

- expose the required Java frontend constraint `vopid`;
- render `VoPAuth1` as `HKVPA`;
- preserve the hbci4java binary prefix behavior by accepting plain frontend
  values and rendering/storing lowlevel `B...` values;
- map process-1 TAN orderhash metadata to `VoPAuth1` / `HKVPA`;
- keep result handling to ordinary job return values and raw status data only.

Do not port `GVVoP` result parsing, VoP polling, HAVE_VOP_RESULT callbacks,
passport persistent VoP data, or automatic queue insertion in this slice.

## Consequences

The Rust port can now represent the final authorization segment for a VoP flow
without committing to the full VoP orchestration model yet.

Callers may queue `VoPAuth` manually with a `vopid`. Future `VoPCheck` work can
reuse the same renderer and constraint surface when the automatic hbci4java flow
is ported.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVVoPAuth.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVVoP.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
