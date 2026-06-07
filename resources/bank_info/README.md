# Bank Info Resources

`blz.properties` is copied from the pinned hbci4java reference checkout and is
loaded by `BankInfoRegistry::bundled()`. Field content is preserved; line
endings are normalized to LF for repository whitespace checks.

Upstream baseline:

- repository: https://github.com/hbci4j/hbci4java
- tag: `hbci4j-core-4.1.11`
- commit: `3b7ce667c73724daa1c836ed7333ed090c21a831`
- upstream path: `src/main/resources/blz.properties`

The file follows the hbci4java bank-info format:

```text
BLZ=name|location|bic|checksum_method|rdh_address|pin_tan_address|rdh_version|pin_tan_version|
```

This is a pinned FinTS/HBCI endpoint snapshot. It is useful for setup and
search, but it is not a promise that a bank still supports a given access path
at runtime. Banks can change URLs, TAN methods, and product access without
changing this crate.
