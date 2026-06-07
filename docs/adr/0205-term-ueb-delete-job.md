# ADR 0205: TermUebDel Job

## Status

Superseded by ADR 0269 and ADR 0276

## Context

`GVTermUebDel` deletes an existing classic, non-SEPA scheduled transfer. In
hbci4java it is a small `HBCIJobImpl` subclass with lowlevel name
`TermUebDel`, a single public constraint `orderid -> id`, and a generic
`HBCIJobResultImpl` result.

The HBCI 300 protocol XML defines `TermUebDel3` / `HKTUL` version 3 for HBCI
3.0. The segment uses `SingleInlandUser4`, so the wire payload still contains
the original scheduled-transfer data plus the `id` of the order to delete.
hbci4java gets that original payload from passport persistent data under
`termueb_<orderid>`, which is written by `GVTermUeb`, `GVTermUebEdit`, and
`GVTermUebList` result handling. If no snapshot exists, hbci4java can be
configured to ignore the problem; otherwise it raises invalid user data.

The Rust port already stores `termueb_<orderid>` snapshots for submitted and
listed classic scheduled transfers.

## Decision

Port `TermUebDel` as an original-near classic scheduled-transfer delete job.

- Expose only the Java highlevel parameter `orderid`, mapped to
  `TermUebDel3.id`.
- Render `TermUebDel3` / `HKTUL` version 3 for HBCI 300 requests.
- Load stored `termueb_<orderid>` snapshot values into the `TermUebDel3`
  lowlevel segment before rendering, matching hbci4java's
  `setLowlevelParam(getName() + "." + key, value)` behavior.
- Keep `orderid` itself as the rendered `id` value and do not persist snapshot
  keys ending in `.id`.
- Reject checked rendering when the referenced `termueb_<orderid>` snapshot is
  missing. Do not port hbci4java's configurable ignore-error escape hatch in
  this slice.
- Keep result handling generic with no typed result and no response content
  mapping because the protocol XML has no `TermUebDelRes*` response segment.
- Do not port `TermUebEdit` in this slice; it has full transfer constraints, a
  typed `GVRTermUebEdit` result, and persistent-data replacement behavior.

## Consequences

The Rust port can delete classic scheduled transfers that were previously
submitted or listed through the port, preserving hbci4java's stateful
`termueb_<orderid>` workflow. Direct deletes without a local snapshot remain a
future compatibility decision because hbci4java's behavior depends on a global
ignore-error setting that is not yet modeled.
