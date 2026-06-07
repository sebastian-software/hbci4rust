# Parser And Generator Goldens

Snapshot date: 2026-06-07.

This page records the v1 evidence for risky offline parser/generator behavior.
It uses the ADR 0254 policy: copied hbci4java fixtures, Rust tests that pin
observable original-near behavior, and explicit limitation entries are all
valid release evidence when they are named here.

It does not declare every possible bank response handled. Malformed
bank-response evidence is tracked in
`docs/reference/malformed-bank-responses.md`.

## Fixture Inventory

Copied hbci4java fixtures in the Rust tree:

```text
tests/fixtures/hbci4java/bank_info/test-bank-info.properties
tests/fixtures/hbci4java/secmech/challengedata.xml
tests/fixtures/hbci4java/secmech/TestMatrixCode-001.txt
tests/fixtures/hbci4java/secmech/TestMatrixCode-002.txt
tests/fixtures/hbci4java/secmech/TestQRCode-001.txt
tests/fixtures/hbci4java/sepa/test-camt-parse-05200102.xml
tests/fixtures/hbci4java/sepa/test-camt-parse-05200108.xml
tests/fixtures/hbci4java/sepa/test-camt-parse-5200108-invalid-saldo.xml
tests/fixtures/hbci4java/sepa/test-camt-parse-5200108-missing-date.xml
tests/fixtures/hbci4java/sepa/test-camt-parse-invalid.xml
tests/fixtures/hbci4java/sepa/test-camt-parse-none.xml
tests/fixtures/hbci4java/sepa/test-camt-ruecklastschrift.xml
tests/fixtures/hbci4java/swift/test-mt940-001.sta
tests/fixtures/hbci4java/swift/test-mt940-002.sta
```

The protocol XML/DTD resources are also copied from the pinned hbci4java
baseline:

```text
resources/protocol/hbci-201.xml
resources/protocol/hbci-210.xml
resources/protocol/hbci-220.xml
resources/protocol/hbci-300.xml
resources/protocol/hbci-plus.xml
resources/protocol/hbci.dtd
```

The bundled bank-info registry uses the pinned hbci4java BLZ table:

```text
resources/bank_info/blz.properties
```

## Coverage Matrix

| Area | Evidence | Current v1 claim | Limitation |
| --- | --- | --- | --- |
| Protocol XML/DTD loading | `resources/protocol/`; `tests/protocol_resources.rs` | Original XML/DTD resources load, parse, expose counts, child refs, defaults, segment lookups, and DTD entity expansion. | This does not remodel the full FinTS spec; v1 keeps original resources and parser behavior. |
| FinTS wire parsing and message rendering | `tests/protocol_wire.rs`; `tests/protocol_message.rs` | Quoted delimiters, binary blocks, defaults, valids, sequence checks, value extraction, signature shell rendering, and repeated elements are pinned with original-near tests. | Broader malformed bank-response behavior is tracked in `docs/reference/malformed-bank-responses.md`. |
| CAMT parsing | `tests/sepa.rs`; copied CAMT fixtures | Version detection, namespace handling, report shell parsing, balances, entries, transaction details, return information, malformed proprietary bank code handling, missing dates, invalid saldo amounts, and copied upstream fixtures are covered. | New bank-specific CAMT quirks need copied fixtures or explicit limitations before release acceptance depends on them. |
| PAIN parsing | `tests/sepa.rs` | Representative PAIN.001 and PAIN.008 parser behavior is covered for old/new transfer fields and direct debit fields. | The upstream PAIN parse fixture set is not fully copied into v1. Add fixtures before claiming broader PAIN parser parity. |
| PAIN generation | `tests/sepa.rs`; `tests/bootstrap.rs`; `docs/reference/migration-examples.md` | Generated PAIN.001/PAIN.008 payloads are checked for Java-compatible fields, sums, mixed-currency rejection, integration with job constraints, and parser roundtrips. | v1 does not claim byte-for-byte Java XML output identity for generated PAIN documents. Add Java-generated golden XML before making byte identity a release claim. |
| SWIFT/MT940 parsing | `tests/swift.rs`; `tests/structures.rs`; copied MT940 fixtures | Umlaut decoding, tag extraction, block splitting, MT940 shell parsing, line parsing, balance correction, storno/year handling, SEPA counter-account mapping, and two copied upstream MT940 fixtures are covered. | MT942 is only covered for the current shell/unbooked-data boundary; full MT942 parser parity is not claimed. |
| Security-mechanism parsers/renderers | `tests/secmech.rs`; copied `challengedata.xml`, QR, and matrix fixtures | Challenge info, HHD version detection, QR, matrix, flicker code parsing, and flicker rendering are covered against upstream-style fixtures and known test cases. | New TAN media payload variants from live banks need replay or fixture coverage before release acceptance depends on them. |
| Bank-info parsing and bundled lookup | `tests/bank_info.rs`; `resources/bank_info/blz.properties`; copied bank-info fixture | Properties parsing, lookup behavior, HBCI version mapping, copied upstream bank-info examples, bundled registry loading, and PinTAN filtering are covered. | The bundled BLZ registry is a pinned upstream snapshot, not a live, always-current bank-support guarantee. |

## Explicit V1 Limitations

These limitations satisfy the release checklist only for the current v1 scope.
They must be revisited if the public API or acceptance bar changes.

- Generated PAIN XML is not promised to be byte-identical to hbci4java output.
- Not all upstream PAIN parse fixtures are copied; current tests cover selected
  representative parser behavior.
- MT942 behavior is only pinned at the current shell/unbooked-data boundary.
- Malformed bank-response acceptance is governed by
  `docs/reference/malformed-bank-responses.md`.
- Bank-specific SCA, TAN media, CAMT, and FinTS quirks discovered during live
  smoke testing must be converted to offline replay fixtures or explicit
  limitations.

## Recheck Commands

```sh
cargo test --test sepa
cargo test --test swift
cargo test --test structures
cargo test --test secmech
cargo test --test bank_info
cargo test --test protocol_resources
cargo test --test protocol_wire
cargo test --test protocol_message
```

The full release-candidate gate still runs `cargo test` and records the complete
output summary after the last release-candidate commit.

## Widening Rules

Before adding or widening parser/generator behavior:

- add copied hbci4java fixtures or generated Java golden artifacts when the
  behavior is risky or hard to infer;
- name any deliberate limitation in this page;
- update the relevant tests and migration docs;
- record a new ADR when the change alters v1 acceptance or public semantics.

## References

- `docs/adr/0007-offline-test-strategy.md`
- `docs/adr/0246-v1-release-checklist.md`
- `docs/adr/0254-parser-generator-golden-artifact-policy.md`
- `docs/architecture/release-checklist.md`
- `docs/architecture/v1-readiness.md`
- `docs/reference/migration-examples.md`
- `tests/fixtures/hbci4java/README.md`
