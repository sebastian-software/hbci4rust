# ADR 0225: FestListAll Job Alias

## Status

Accepted

## Context

`GVFestListAll` in hbci4java extends `GVFestList` and returns the same
lowlevel name, `FestList`. It therefore uses the same FinTS 3.0 segment as
`GVFestList`: `FestList4` / `HKFGB` version 4 with `FestListRes4` / `HIFGB`
version 4 responses.

The only constructor-level behavioral difference is the `dummy -> allaccounts`
constraint default:

- `GVFestList` defaults `dummy` to `"N"`;
- `GVFestListAll` defaults `dummy` to `"J"`.

`GVFestListAll#redoAllowed()` returns true. It inherits the result shape and
parsing behavior from `GVFestList`, so there is no separate hbci4java result
class.

## Decision

Port `FestListAll` as an original-near public job alias.

- Add `FestListAll` to the PinTAN job registry.
- Keep the Java frontend parameters: `my.number`, `my.subnumber`, `my.blz`,
  `my.country`, and `dummy`.
- Map all constraints to the existing `FestList4` paths.
- Default `dummy` / `FestList4.allaccounts` to `"J"`.
- Reuse the existing `FestList` renderer, order-hash source mapping, result
  parser, and raw `content.*` result data handling.
- Return typed result data as `HbciJobResultData::FestList`, matching the
  shared hbci4java result class.

## Consequences

`FestListAll` becomes available by its hbci4java job name while avoiding a
duplicate protocol implementation. Consumers can explicitly request all
fixed-term deposit accounts with `new_job("FestListAll")`, and tests can
distinguish the public job name from the shared lowlevel `HKFGB` wire shape.

## References

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVFestListAll.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVFestList.java`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
