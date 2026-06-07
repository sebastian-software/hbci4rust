# ADR 0140: Callback TAN Method Selection

## Status

Accepted

## Context

hbci4java asks the user for a PinTAN security mechanism through
`AbstractPinTanPassport.chooseTANMethod(...)` when automatic selection cannot
choose safely. It passes callback reason `HBCICallback.NEED_PT_SECMECH`, message
`*** Select a pintan method from the list`, data type `TYPE_TEXT`, and a
pipe-separated `id:name` option string in `retData`. The callback must replace
that string with the selected method id. hbci4java then validates that the
returned id is one of the offered options and throws `InvalidUserDataException`
otherwise.

ADR 0139 added a deterministic Rust helper that returns `NeedsUserSelection`
instead of inventing a default choice. The handler still ignores that result.

## Decision

Add callback-assisted TAN-method selection for ambiguous PinTAN mechanisms:

- when `PinTanPassport::determine_tan_method()` returns `NeedsUserSelection`,
  call the configured async callback if one exists;
- use callback reason `NeedPtSecMech` and data type `Select`;
- pass the original-near `id:name|id:name` option string as `current_value`;
- accept only callback responses whose selected id is one of the offered
  options;
- persist the selected method through the passport, matching hbci4java's
  `setCurrentTANMethod(...)` after successful selection;
- return a callback error when the callback returns no value or an unsupported
  id.

Do not add a blocking/stdin fallback, dialog restart behavior, or
`tanMethodAutoSelected` in this slice.

## Consequences

The Rust handler can now progress past ambiguous but known PinTAN method lists
when the application supplies an async callback. Headless callers without a
callback still get the previous non-mutating behavior until a higher-level API
for unresolved selections exists.

Remaining work:

- surface unresolved selections without global callbacks;
- port dialog restart after changed `3920` method lists;
- add TAN media and final TAN entry callbacks;
- connect selected methods to full HKTAN queue patching and SCA flows.

## Links

- `src/callback.rs`
- `src/manager/handler.rs`
- `src/passport/pintan.rs`
- Upstream: `org.kapott.hbci.passport.AbstractPinTanPassport.chooseTANMethod`
- Upstream: `org.kapott.hbci.callback.HBCICallback.NEED_PT_SECMECH`
