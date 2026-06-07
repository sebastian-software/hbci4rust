# ADR 0138: User Two-Step Mechanism Import

## Status

Accepted

## Context

hbci4java keeps two PinTAN mechanism lists:

- bank-supported mechanisms extracted from BPD (`tanMethodsBank`);
- user-allowed mechanisms extracted from return code `3920`
  (`tanMethodsUser`).

`AbstractPinTanPassport.check3920(...)` searches message status return values
for known return code `3920`, collects all return-value parameters, de-duplicates
them, and replaces the user-allowed list when the received list is non-empty.
It may then reselect the current TAN method and request a dialog restart.

The Rust port already imports bank-supported two-step mechanisms from BPD but
does not persist user-allowed mechanisms yet.

## Decision

Add an original-near user mechanism import to the PinTAN passport:

- store `allowed_twostep_mechanisms` in `PinTanPassportData`;
- import it from `HbciMsgStatus` return values with code `3920`;
- collect all non-empty return-value parameters;
- de-duplicate deterministically;
- replace the stored list only when the newly received list is non-empty;
- call this import after dialog initialization response parsing.

Do not port dialog restart or automatic TAN-method reselection in this slice.
That behavior depends on the still-missing full TAN-method selection flow.

## Consequences

The Rust port now preserves the split between bank-supported and user-allowed
PinTAN mechanisms and can use that data in later TAN-method selection work.

Remaining work:

- port hbci4java's automatic/user-assisted TAN-method selection;
- decide how to represent dialog restart requests in the async handler;
- use allowed mechanisms when choosing the active `tan_method`;
- persist/import legacy passport `twostepMechs` only if Java passport import is
  ever brought back into scope.

## Links

- `src/passport/pintan.rs`
- `src/manager/handler.rs`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.check3920`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.getAllowedTwostepMechanisms`
