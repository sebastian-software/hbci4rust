# ADR 0184: SEPA Account Transfer Job

## Status

Accepted

## Context

`GVUmbSEPA` is hbci4java's high-level job for SEPA account transfers
(`Umbuchungen`). It extends `AbstractSEPAGV`, uses `UmbSEPA` as its lowlevel
name, and deliberately returns `UebSEPA` from `getPainJobName()` because the
underlying SEPA document is the same `pain.001` credit-transfer family used by
`GVUebSEPA`.

The pinned protocol resources define `UmbSEPA1` / `HKCUM` in both
`hbci-plus.xml` and `hbci-300.xml`. The request segment contains the source
account, PAIN descriptor, and PAIN binary block. There is no dedicated
`UmbSEPARes1` response segment; hbci4java therefore uses the default
`HBCIJobResultImpl` result shape.

`GVUmbSEPA` exposes the same single-transfer SEPA dummy parameters as
`GVUebSEPA`, except that it does not add the `batchbook` dummy constraint.

## Decision

Port `UmbSEPA` as the next single-transfer PinTAN slice:

- expose original-near constraints for `UmbSEPA1`, matching `GVUmbSEPA`
  source-account fields, `_sepadescriptor`, `_sepapain`, SEPA dummy
  parameters, indexed destination / amount / usage fields, `sepaid`,
  `pmtinfid`, `endtoendid`, and `purposecode`;
- render `UmbSEPA1` as `HKCUM`, preserving the segment order account,
  descriptor, and PAIN binary block;
- use the existing ADR 0177 `pain.001.001.02` single-transfer generator when
  `_sepapain` is absent;
- keep `UmbSEPA` without a typed result variant until upstream response data is
  in scope.

Do not port BPD parameter handling for `UmbSEPAPar1`, multi-transfer account
transfers, newer PAIN.001 versions, or account-transfer-specific status
semantics in this slice.

## Consequences

The Rust port can now replay-test SEPA account transfers with the same PAIN
generator used by `UebSEPA` and `InstUebSEPA`, while keeping the implementation
close to hbci4java's current request-only job shape.

The implementation intentionally duplicates the single-transfer constraints for
the new lowlevel segment instead of introducing a shared transfer-job
abstraction before more variants are ported.

## Links

- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/GVUmbSEPA.java`
- `target/reference/hbci4java/src/main/java/org/kapott/hbci/GV/AbstractSEPAGV.java`
- `target/reference/hbci4java/src/main/resources/hbci-plus.xml`
- `target/reference/hbci4java/src/main/resources/hbci-300.xml`
- `docs/adr/0177-pain001-transfer-generator.md`
- `docs/adr/0178-ueb-sepa-job.md`
- `docs/adr/0183-instant-sepa-transfer-job.md`
