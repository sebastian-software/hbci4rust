# Migration Examples

These examples show high-risk v1 PinTAN/HBCI-Plus job shapes using the public
crate-root API. They keep hbci4java job names and parameter keys intact.

The examples stop after queue preparation. In a live client, use the handler
flow from `docs/reference/java-to-rust-mapping.md`: open the dialog, queue the
job, call `execute_with_tan2step().await`, then close the dialog.

`tests/public_api.rs` checks these shapes against the current public API.

## Shared Setup

```rust
use hbci4rust::{
    HbciHandler, HbciResult, Konto, PinTanPassport, PinTanPassportData,
};

fn passport() -> PinTanPassport {
    PinTanPassport::new(PinTanPassportData {
        country: "DE".to_owned(),
        blz: "12345678".to_owned(),
        host: Some("https://fints.example.test/fints".to_owned()),
        user_id: "user".to_owned(),
        customer_id: Some("customer".to_owned()),
        ..PinTanPassportData::default()
    })
}

fn account() -> Konto {
    Konto {
        country: Some("DE".to_owned()),
        blz: Some("12345678".to_owned()),
        number: Some("1234567890".to_owned()),
        bic: Some("TESTDEFFXXX".to_owned()),
        iban: Some("DE02123456780012345678".to_owned()),
        ..Konto::default()
    }
}
```

## MT940/MT942 Statement: `KUmsAll`

hbci4java job name: `KUmsAll`

Important parameter keys:

- `my.*` for the account;
- `startdate` and `enddate` as ISO dates;
- `maxentries` as an optional page-size hint.

```rust
# use hbci4rust::{HbciHandler, HbciResult, Konto, PinTanPassport};
# fn account() -> Konto { Konto { iban: Some("DE02123456780012345678".to_owned()), bic: Some("TESTDEFFXXX".to_owned()), ..Konto::default() } }
fn queue_kums_all(handler: &mut HbciHandler, account: &Konto) -> HbciResult<()> {
    let mut job = handler.new_job("KUmsAll")?;
    job.set_param_account("my", account);
    job.try_set_param_date("startdate", "2026-06-01")?;
    job.try_set_param_date("enddate", "2026-06-06")?;
    job.try_set_param_int("maxentries", 25)?;
    handler.try_add_to_queue(job)
}
```

## CAMT Statement: `KUmsAllCamt`

hbci4java job name: `KUmsAllCamt`

Important parameter keys:

- `my.*` for the account;
- `startdate`, `enddate`, and `maxentries`;
- `offset` when continuing from a bank-provided cursor;
- `suppformat` defaults to the currently ported CAMT 052 descriptor.

```rust
# use hbci4rust::{HbciHandler, HbciResult, Konto};
fn queue_kums_all_camt(handler: &mut HbciHandler, account: &Konto) -> HbciResult<()> {
    let mut job = handler.new_job("KUmsAllCamt")?;
    job.set_param_account("my", account);
    job.try_set_param_date("startdate", "2026-06-01")?;
    job.try_set_param_date("enddate", "2026-06-06")?;
    job.try_set_param_int("maxentries", 25)?;
    job.set_param("offset", "CURSOR");
    handler.try_add_to_queue(job)
}
```

## SEPA Credit Transfer: `UebSEPA`

hbci4java job name: `UebSEPA`

Important parameter keys:

- `src.iban`, `src.bic`, and `src.name` for the sender;
- `dst.iban`, `dst.bic`, and `dst.name` for the recipient;
- `btg.value` and optional `btg.curr`;
- `usage`;
- `sepaid` to influence the generated PAIN message and payment info id.

```rust
# use hbci4rust::{HbciHandler, HbciResult};
fn queue_ueb_sepa(handler: &mut HbciHandler) -> HbciResult<()> {
    let mut job = handler.new_job("UebSEPA")?;
    job.try_set_param("src.iban", "DE02123456780012345678")?;
    job.try_set_param("src.bic", "TESTDEFFXXX")?;
    job.try_set_param("src.name", "Sender Name")?;
    job.try_set_param("dst.iban", "DE99123456780098765432")?;
    job.try_set_param("dst.bic", "DEUTDEDB277")?;
    job.try_set_param("dst.name", "Receiver Name")?;
    job.try_set_param("btg.value", "12.30")?;
    job.try_set_param("usage", "Invoice 4711")?;
    job.try_set_param("sepaid", "SEPA-UEB")?;
    handler.try_add_to_queue(job)
}
```

The Rust port generates the `pain.001.001.02` payload during constraint
verification when `_sepapain` is not supplied.

## SEPA Direct Debit: `LastSEPA`

hbci4java job name: `LastSEPA`

Important parameter keys:

- `src.*` for the creditor account;
- `dst.*` for the debtor account;
- `btg.value` and optional `btg.curr`;
- `creditorid`, `mandateid`, `manddateofsig`, and `targetdate`;
- `usage` and `sepaid`;
- `sequencetype` defaults to `FRST`, and `type` defaults to `CORE`.

```rust
# use hbci4rust::{HbciHandler, HbciResult};
fn queue_last_sepa(handler: &mut HbciHandler) -> HbciResult<()> {
    let mut job = handler.new_job("LastSEPA")?;
    job.try_set_param("src.iban", "DE02123456780012345678")?;
    job.try_set_param("src.bic", "TESTDEFFXXX")?;
    job.try_set_param("src.name", "Creditor Name")?;
    job.try_set_param("dst.iban", "DE99123456780098765432")?;
    job.try_set_param("dst.bic", "DEUTDEDB277")?;
    job.try_set_param("dst.name", "Debtor Name")?;
    job.try_set_param("btg.value", "12.30")?;
    job.try_set_param("usage", "Direct debit usage")?;
    job.try_set_param("sepaid", "SEPA-LAST")?;
    job.try_set_param("creditorid", "DE98ZZZ09999999999")?;
    job.try_set_param("mandateid", "MND-123")?;
    job.try_set_param("manddateofsig", "2026-01-02")?;
    job.try_set_param("targetdate", "2026-03-15")?;
    handler.try_add_to_queue(job)
}
```

The Rust port generates the `pain.008.001.01` payload during constraint
verification when `_sepapain` is not supplied.

## Boundaries

These examples are not a new builder API. They intentionally use the public
original-near `HbciJob` string setters so hbci4java migrations can keep job
names and parameter keys visible.
