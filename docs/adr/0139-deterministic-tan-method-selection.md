# ADR 0139: Deterministic TAN Method Selection Helper

## Status

Accepted

## Context

hbci4java keeps the active PinTAN method in `tanMethod` and selects it through
`AbstractPinTanPassport.getCurrentTANMethod(...)`. With the automatic strategy,
`determineTanMethod()` compares:

- bank-supported two-step methods from BPD (`tanMethodsBank`);
- user-allowed methods from return code `3920` (`tanMethodsUser`);
- the current cached TAN method;
- whether one-step TAN method `999` is still allowed by BPD.

When the result is unambiguous, hbci4java stores the selected two-step method.
When only one-step is usable as a bootstrap fallback, it returns `999` without
storing it because that cannot be the final PSD2/SCA method. When more than one
two-step method is possible, hbci4java asks the user and may later request a
dialog restart.

The Rust port already has bank mechanisms and the imported user mechanism list,
but it does not yet have the full callback-assisted TAN-method selection flow.

## Decision

Add a limited, deterministic selection helper to the PinTAN passport:

- expose the current TAN method and whether one-step method `999` is allowed;
- keep one-step `999` as an explicit fallback result and do not persist it;
- select and persist a two-step method only when hbci4java would do so without
  asking the user:
  - exactly one user-allowed bank method exists;
  - or the current method is still present in a multi-method user list;
- return a `NeedsUserSelection` result with sorted options when hbci4java would
  call its TAN-method callback;
- call the helper after dialog initialization imports BPD and `3920` data, but
  let ambiguous selections remain unresolved.

Do not port `chooseTANMethod(...)`, `tanMethodAutoSelected`, dialog restart
requests, or manual callback selection in this slice.

## Consequences

The handler can now preserve hbci4java's safe automatic cases without inventing
a Rust-only selection policy. Ambiguous choices remain visible for the future
async callback flow.

Remaining work:

- port callback-assisted TAN method selection;
- model hbci4java's dialog restart request after changed `3920` data;
- port the manual/ask strategy and `tanMethodAutoSelected` behavior if needed;
- feed the selected method into full HKTAN queue patching and final TAN
  submission.

## Links

- `src/passport/pintan.rs`
- `src/manager/handler.rs`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.determineTanMethod`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.askForTanMethod`
