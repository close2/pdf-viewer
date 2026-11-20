# Round 2133 — what a crypt filter name binds: audit 2131's groups A, B and D

A fix round in the `audit` worktree for review 2131 (§7.6): B01 to B06, D01 to D04. T01 waits on
Q953 and is untouched.

## What changed

- Group A (ADR 3233): one predicate, `crypt::is_standard_filter_name`, is Table 20's "shall be
  ignored" for Table 26's names, and `crypt_filters`, `key_length`, `named_method` and the
  public-key `filter_lists` all ask it (B02, B04). `key_length` reads the filter `/StmF` or
  `/StrF` names and refuses a bit count that is not a multiple of 8 (D01). `named_method` returns
  `None` for a name `/CF` does not define, and the stream is refused (B03, D04). The writer stops
  with `SerializeError::UndefinedCryptFilter` for the same name.
- Group B (ADR 3234): `/EncryptMetadata false` leaves only the stream the catalog's `/Metadata`
  names in the clear, reader and writer (B01). A `/Type /XRef` stream's dictionary is not
  decrypted (B05). An untyped stream is an embedded file stream when a file specification's `/EF`
  or `/RF` names it. The references are collected once, without decryption, the first time an
  untyped stream asks in a document whose `/EFF` and `/StmF` differ (B06).
- Group D: `crypt.rs`'s module doc states what `pdf_pki::enveloped` refuses (D02), and
  `serialize.rs` quotes Table 25 as `doc/md/` prints it (D03).

## Ledger and manifest

Rows 7.6.2, 7.6.3.2, 7.6.4.2, 7.6.4.3.2, 7.6.5.2 and 7.6.6 stay `implemented`. The findings
against them are fixed, and they list the audit tests. The notes of 7.6.2 and 7.6.6 now state the
construction. The manifest loses the six 2131 entries, 12 to 6. 2131's `findings.json` marks ten
findings fixed.

**Gates.** Over `-p pdf-syntax -p pdf-transform -p pdf-model`: fmt exit 0 after one `cargo fmt`.
clippy `-D warnings --all-targets` exit 0. `pdf-sandbox --bins` exit 0. nextest `--lib --bins
--tests`: 4557 run, 4557 passed. The difference against the manifest is 0 and 0.
`cargo test -p pdf-syntax`: 406 passed, 0 failed. rustdoc `-D warnings --document-private-items`
exit 0. `cd fuzz && cargo check --bins` exit 0 (`SerializeError` gained a variant). `cargo test -p
conformance --no-fail-fast`: 478 passed, 0 failed.

## Habit

A rule the clause states about one stream ("the document-level metadata stream", "embedded file
streams") is keyed on which stream it is. A `/Type` is optional, and other streams state it too,
so a rule keyed on the `/Type` alone covers the wrong set.
