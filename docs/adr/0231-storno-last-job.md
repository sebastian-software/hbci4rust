# 0231 Port StornoLast As Classic Direct Debit Objection

## Status

Accepted

## Context

`GVStornoLast` in hbci4java implements classic domestic direct debit objection.
It uses the lowlevel job name `LastObjection` and returns the generic
`HBCIJobResultImpl`.

In FinTS 3.0 the protocol XML contains two request segment versions:

- `LastObjection1` / `HKLSW` version 1
- `LastObjection2` / `HKLSW` version 2

`LastObjection2` adds optional `usage` and `orderid` fields. hbci4java exposes
`orderid`, but it does not add frontend constraints for `usage` in
`GVStornoLast`.

The hbci4java constructor constraints map to `LastObjection` fields:

- `my.* -> My.*`
- `other.* -> Other.*`
- `btg.value -> BTG.value`
- `btg.curr -> BTG.curr`
- `name -> name`
- `date -> Timestamp.date`
- optional `name2`, `primanota`, `time`, and `orderid`

`verifyConstraints()` checks both `my` and `other` account CRC values.
Challenge metadata for `HKLSW` uses `Other.number`, `BTG.value`, and
`BTG.curr` for older HHD specs, while HHD 1.4 has no GV class mapping.

## Decision

Port `StornoLast` as the next original-near PinTAN job slice:

- expose frontend job name `StornoLast`;
- map constraints to `LastObjection2`;
- render `LastObjection2` as `HKLSW` version 2 with `My`, `Timestamp`, `BTG`,
  `Other`, `name`, optional `name2`, optional `primanota`, and optional
  `orderid`;
- keep `usage` absent from the public job surface in this slice because
  hbci4java does not expose it for `GVStornoLast`;
- register `HKLSW` orderhash metadata for the rendered segment;
- keep the result generic, with no typed result parser and no `content.*` data.

## Consequences

The Rust port can replay-test classic direct debit objections with the current
signed PinTAN `CustomMsg` path. The implementation stays close to the Java job
surface instead of opportunistically exposing extra protocol fields.

If a later bank replay requires `LastObjection2.usage`, add it through a new ADR
that explicitly records the deviation from hbci4java's `GVStornoLast`
constructor constraints.

## References

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVStornoLast.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `target/reference/hbci4java/src/main/resources/challengedata.xml`
