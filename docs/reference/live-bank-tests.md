# Optional Live Bank Tests

The repository's acceptance tests are offline-only. Live bank tests are manual
smoke hooks for developers who explicitly want to check the current PinTAN
dialog shell against a real FinTS endpoint.

## Safety Model

- Live tests are `#[ignore]` and do not run in `cargo test`.
- Live tests also require `HBCI4RUST_LIVE_ENABLE=1`.
- Credentials are read only from environment variables.
- Credential values are not logged by the test.
- The initial live hook only opens and closes a dialog. It does not submit
  payments, direct debits, standing orders, or account jobs.

## Required Variables

```sh
export HBCI4RUST_LIVE_ENABLE=1
export HBCI4RUST_LIVE_HOST="https://..."
export HBCI4RUST_LIVE_BLZ="..."
export HBCI4RUST_LIVE_USER_ID="..."
export HBCI4RUST_LIVE_PIN="..."
```

## Optional Variables

```sh
export HBCI4RUST_LIVE_COUNTRY="DE"
export HBCI4RUST_LIVE_CUSTOMER_ID="..."
export HBCI4RUST_LIVE_FILTER="..."
export HBCI4RUST_LIVE_HBCI_VERSION="300"
export HBCI4RUST_LIVE_TAN_METHOD="..."
export HBCI4RUST_LIVE_TAN_MEDIA="..."
export HBCI4RUST_LIVE_TAN="..."
```

`HBCI4RUST_LIVE_TAN` is only used if the bank asks for a TAN during the dialog
smoke. Most manual runs should start without it; if a bank requires an SCA step,
the first run will fail with the missing variable name and no secret value.

## Run

```sh
cargo test --test live_bank -- --ignored
```

Without `HBCI4RUST_LIVE_ENABLE=1`, the ignored test exits successfully without
opening a network connection. This keeps accidental local invocations harmless.

## Current Hook

`live_pintan_dialog_init_and_close_from_env`:

- builds a `PinTanPassport` from the environment;
- installs an async callback that supplies PIN/TAN selection values from the
  environment;
- calls `HbciHandler::init().await`;
- calls `HbciHandler::close().await`.

## References

- `docs/adr/0236-optional-live-bank-test-hooks.md`
- `tests/live_bank.rs`
- `src/comm/mod.rs`
- `src/manager/handler.rs`
