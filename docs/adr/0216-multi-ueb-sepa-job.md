# 0216 Port MultiUebSEPA With Indexed PAIN.001 Transfers

## Status

Accepted

## Context

`GVMultiUebSEPA` in hbci4java implements SEPA bulk transfers. It extends
`GVUebSEPA`, but uses lowlevel job name `SammelUebSEPA` instead of `UebSEPA`.
The HBCI 300 protocol resource defines request segment `SammelUebSEPA1` with
code `HKCCM` version 1. The segment contains the source account, optional
`Total`, optional `singletransfers`, `sepadescr`, and `sepapain`.

hbci4java keeps the PAIN job name as `UebSEPA` and adds three constraints on top
of the inherited transfer constraints:

- `batchbook` mapped to `sepa.batchbook`, default empty string;
- `Total.value` mapped to `Total.value`, required;
- `Total.curr` mapped to `Total.curr`, required.

During SEPA generation, hbci4java uses indexed frontend values such as
`dst.iban[0]`, `btg.value[1]`, and `usage[1]` to generate multiple
`CdtTrfTxInf` entries. The Java generator sets `NbOfTxs` to the number of
indexed transfers, computes `CtrlSum` from all `btg.value` entries, and rejects
mixed currencies.

The Rust port already has a `UebSEPA` single-transfer path, indexed parameter
setters, and a PAIN.001.001.02 generator. It does not yet generate multi-entry
PAIN.001 documents or render `SammelUebSEPA1`.

## Decision

Port `MultiUebSEPA` as an original-near SEPA bulk-transfer job:

- expose frontend job name `MultiUebSEPA`;
- map constraints to `SammelUebSEPA1` while keeping Java frontend names;
- keep `_sepadescriptor` defaulted to `PAIN_001_001_02`;
- support both raw `_sepapain` and generated PAIN from indexed transfer
  parameters;
- generate PAIN.001.001.02 documents with one `CdtTrfTxInf` per indexed
  transfer;
- compute `Total.value` and `Total.curr` from generated transfer data when not
  provided explicitly;
- reject mixed currencies during generated multi-transfer PAIN creation;
- render request segment `HKCCM` and leave result data as basic status only,
  matching hbci4java's plain `HBCIJobResultImpl` result object for this job.

Do not add `singletransfers` as a public frontend parameter in this slice. The
upstream Java constructor does not add a constraint for it, so it remains
unexposed for now.

## Consequences

This moves another registry-visible PinTAN job from "known name" to runnable
offline parity and reuses the existing SEPA transfer surface instead of adding a
new public API style.

The PAIN.001 generator grows from single-transfer to single-or-multi transfer
support. Later SEPA bulk jobs such as `TermMultiUebSEPA` can reuse the same
generator and total calculation rather than inventing another path.
