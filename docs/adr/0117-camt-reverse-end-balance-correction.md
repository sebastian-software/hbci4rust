# ADR 0117: CAMT Reverse End Balance Correction

## Status

Accepted

## Context

`ParseCamt05200102#parse(...)` contains an Apo-Bank special case after all CAMT
entries have been parsed. When the parsed day has no start-balance timestamp
but does have an end-balance timestamp, hbci4java walks the transaction lines
backwards from the end balance:

- set the last line balance to the end balance;
- subtract the last line amount from the working balance;
- repeat for the preceding lines.

The existing CAMT port currently starts running balances at zero when no start
balance is available. That keeps parsing deterministic but does not match this
hbci4java correction behavior.

## Decision

Port the reverse end-balance correction for CAMT report parsing.

Treat Rust `start == None` or `start.date == None` as hbci4java's missing
`tag.start.timestamp`, and require `end.date != None` before correcting. Only
replace each line's saldo value; keep line dates, currencies, amounts, and the
day-level start/end fields unchanged.

Do not synthesize a day-level start balance from the corrected line sequence in
this slice.

## Consequences

CAMT reports with only a usable end balance now produce hbci4java-like line
balances instead of zero-based running balances. Reports with a dated start
balance keep the existing forward-running balance behavior.
