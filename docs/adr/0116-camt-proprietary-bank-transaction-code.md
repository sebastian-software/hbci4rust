# ADR 0116: CAMT Proprietary Bank Transaction Code

## Status

Accepted

## Context

ADR 0115 ports CAMT return information. The next block in
`ParseCamt05200102#createLine(...)` reads `TxDtls/BkTxCd/Prtry/Cd`.

hbci4java treats this field as a Sparkasse-specific convention: when the code
contains `+`, it splits the string on `+`; only if the split has exactly four
parts, it maps:

- part 2 to `line.gvcode`;
- part 3 to `line.primanota`;
- part 4 to `line.addkey`.

Malformed or differently shaped proprietary codes are ignored.

## Decision

Port this proprietary CAMT bank transaction-code mapping as-is for the first
transaction detail selected by the current CAMT parser.

Use Java-like `String.split("\\+")` behavior for this check, including
discarding trailing empty parts before testing the part count.

Do not infer meanings for other code formats and do not remodel transaction
classification in this slice.

## Consequences

CAMT parsing now carries Sparkasse-style GV-code metadata into the same
`GvrKUmsLine` fields used by MT940 parsing, while keeping unsupported
proprietary code shapes inert.
