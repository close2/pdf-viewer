# Round 2131 — the second specification audit: §7.6, encryption

An audit round in the `audit` worktree, made in the shape of review 2038 and audit 2128, with
2128's habit — what binds to what — turned on §7.6. It fixes nothing. No ledger row moves.

## What it found

- `doc/reviews/2131-specification-audit-7-6.md` and its directory: §7.6.1 to §7.6.7 read against
  `pdf_syntax::crypt`, `crypt/public_key.rs`, `document/encryption.rs`, the writer in
  `serialize.rs` and `pdf_pki::enveloped`. Eleven findings: 6 B, 1 T, 4 D.
- None lets a forged document pass. The six are about which name or which stream a rule binds:
  B01 every `/Type /Metadata` stream in the clear, not only the document-level one; B02
  `key_length` reads the first `/CF` entry, an `/Identity` one included; B03 an undefined `/Crypt`
  name read as Identity, ciphertext shown as data; B04 the public-key handler reads an `/Identity`
  `/CF` entry; B05 a cross-reference stream's dictionary decrypted, `/ID` included; B06 an embedded
  file stream known only by its optional `/Type`, so `/EFF` misses an untyped attachment.
- T01: an unverifiable revision 6 `/Perms` opens under the plaintext `/P` untested; Q953 asks what
  it should do.
- Six failing `*_spec_audit` tests in `crates/pdf-syntax/tests/encryption_spec_audit.rs`, each with
  a control, on documents encrypted by the test from Algorithms 1 to 5; listed in the audit's
  `expected-failures.json` and appended to `2038-specification-audit/expected-failures-current.json`.

## Ledger

34 §7.6 rows: 33 `implemented`, 7.6.7 `writer-side`, before and after. B findings contradict
7.6.2, 7.6.3.2, 7.6.4.2, 7.6.4.3.2, 7.6.5.2 and 7.6.6, and their aggregates.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets` over `-p pdf-syntax` exit 0 on the
second run; the first found a doc comment the helper copy had orphaned. `pdf-sandbox --bins` exit 0.
nextest `--lib --bins --tests` of `pdf-syntax` under `xvfb-run -a`: 406 run, 400 passed, 6 failed,
1 skipped; the 6 are the manifest's new entries, and `audit-diff.py` prints "failed 6, expected 6".
rustdoc `-D warnings --document-private-items` exit 0. `cargo test -p conformance --no-fail-fast`
exit 0 on the second run; the first named a round in the test file's header and wanted this
paragraph. No public type changed, so fuzz is not owed.

## Habit

A rule keyed on a name binds wherever that name is read. 2038-B07 made `crypt_filters` ignore an
`/Identity` `/CF` entry; `key_length` and the public-key `filter_lists` read the same dictionary
and kept reading it. When a fix makes a reader skip an entry, grep for every reader of that entry.
