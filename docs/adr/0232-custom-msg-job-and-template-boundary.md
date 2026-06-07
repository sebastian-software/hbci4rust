# 0232 Port CustomMsg And Keep Template Lowlevel Fallback Out Of V1

## Status

Accepted

## Context

hbci4java has two adjacent concepts with similar names:

- `GVCustomMsg` is a concrete job class. It uses lowlevel name `CustomMsg`,
  segment code `HKKDM`, and generic `HBCIJobResultImpl`.
- `GVTemplate` is not a concrete bank job. `HBCIHandler.newLowlevelJob(gvname)`
  creates it dynamically for arbitrary lowlevel segment names and adds
  constraints on demand in `setParam`.

The Rust v1 plan chose a static PinTAN-relevant job registry instead of a free
lowlevel job surface. That keeps the original-near port testable from known Java
classes and avoids exposing arbitrary FinTS segments before the protocol,
security, and result handling boundaries are ready.

For FinTS 3.0 the protocol XML defines `CustomMsg5` as `HKKDM` version 5:

- optional `KTV`;
- required `msg` with maxsize 2048;
- optional `betreff` and `recpt`;
- no `curr` field.

`GVCustomMsg` still exposes `my.curr -> curr` with default `EUR`, matching older
protocol versions such as `CustomMsg3`. It also checks `ParCustomMsg.maxlen`
from bank restrictions when present. The current Rust job object has no
BPD-bound restriction lookup at parameter-set time.

## Decision

Port `GVCustomMsg` as a normal static v1 job:

- expose frontend job name `CustomMsg`;
- map the Java constraints to `CustomMsg5` lowlevel paths;
- preserve the Java `my.curr` constraint and default `EUR` in the job surface,
  but do not render it for FinTS 3.0 because `CustomMsg5` has no `curr` element;
- render `HKKDM` version 5 inside the signed PinTAN `CustomMsg` message envelope;
- check the `my` account CRC like hbci4java;
- keep the result generic with no typed `content.*` parser;
- rely on XML/wire validation for the static `msg` maxsize for now and defer
  dynamic BPD `ParCustomMsg.maxlen` enforcement.

Keep `GVTemplate` out of the v1 public API and registry. If the port later needs
lowlevel jobs, add an explicit lowlevel API ADR instead of smuggling arbitrary
segment names through `new_job`.

## Consequences

The final concrete hbci4java `GV*` job in v1 scope can be executed through the
same replayable PinTAN path as the other ported jobs.

The Rust job registry remains intentionally narrower than Java's
`newLowlevelJob`: unknown segment names still fail fast as out of scope. This is
a deliberate v1 safety boundary, not a permanent statement that lowlevel jobs
will never be supported.

If real bank fixtures require BPD-specific `maxlen` checks before rendering,
add that through a new ADR at the point where job restrictions are attached to
queued jobs.

## References

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVCustomMsg.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVTemplate.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/manager/HBCIHandler.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
