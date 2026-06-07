# 0230 Port MultiLast As Classic Bulk Direct Debit

## Status

Superseded by ADR 0266 and ADR 0272

## Context

`GVMultiLast` in hbci4java implements classic domestic bulk direct debits. It
extends `AbstractMultiGV`, uses the lowlevel job name `SammelLast`, and returns
the generic `HBCIJobResultImpl`.

In FinTS 3.0 the request segment is `SammelLast6` with code `HKSLA`, version 6.
The segment uses the same `SammelUser3` entity shape as `SammelUeb6`:

- `SegHead`
- `KTV`
- binary `data`

`GVMultiLast` exposes the same constructor constraints as `GVMultiUeb`, mapped
to `SammelLast6`:

- `data -> data`, required
- `my.country -> KTV.KIK.country`, default `DE`
- `my.blz -> KTV.KIK.blz`, required
- `my.number -> KTV.number`, required
- `my.subnumber -> KTV.subnumber`, default empty

Like `GVMultiUeb`, `setParam("data", value)` prefixes the stored lowlevel value
with `B`, so the DTAUS payload is rendered as FinTS binary data. The job checks
the `my` account CRC in `verifyConstraints()`.

The protocol XML has no `SammelLastRes` response segment. Challenge metadata for
`HKSLA` uses `AbstractMultiGV`-style aggregate DTAUS values (`sumOthers`,
`sumValue`, `sumCurr`, and `sumCount`), but this slice does not parse DTAUS for
challenge parameter extraction.

## Decision

Port `MultiLast` as the next original-near PinTAN job slice:

- expose frontend job name `MultiLast`;
- map constraints to `SammelLast6`;
- preserve hbci4java's `B...` binary lowlevel storage for `data`;
- render `SammelLast6` as `HKSLA` with the domestic `KTV` account and binary
  `data`;
- register `HKSLA` orderhash metadata for the rendered segment;
- keep the result generic, with no typed result parser and no `content.*` data;
- do not generate or parse DTAUS in this slice.

## Consequences

The Rust port can replay-test classic bulk direct debit submission with the same
wire-level shape as hbci4java while keeping the implementation narrow and close
to `MultiUeb`.

TAN challenge aggregate values for `HKSLA` remain a later follow-up. If needed,
they should be added through a shared `AbstractMultiGV`-near DTAUS parser rather
than special-casing `MultiLast`.

## References

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVMultiLast.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/AbstractMultiGV.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `target/reference/hbci4java/src/main/resources/challengedata.xml`
