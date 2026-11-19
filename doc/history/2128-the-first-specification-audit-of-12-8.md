# Round 2128 — the project's first specification audit: §12.8, digital signatures

An audit round in the `gwg` worktree, asked for by the owner on 2026-11-19 ("start audits") and
made in the shape of review 2038. It fixes nothing. No ledger row moves.

## What it found

- `doc/reviews/2128-specification-audit-12-8.md` and its directory: 232 modal sentences of §12.8
  read against `pdf-signature`, `pdf-pki`'s DER reader and `viewer-core`'s report. Thirteen
  findings: 7 B, 2 T, 4 D.
- The two serious ones are about what a CMS value binds. B01: an `adbe.pkcs7.sha1` value's
  encapsulated SHA-1 can be rewritten after signing and the value still reads `Unchanged` and
  `Verified`, because nothing compares `message-digest` with the content. B02: an
  `adbe.pkcs7.detached` value that encapsulates unrelated bytes, with no signed attributes, reads
  `UnderTheSignersKey` and `Verified`, and the document is never digested. Both clear
  `Verdict::reached`'s first two questions.
- B03 (no content-type attribute check, §12.8.3.4.3 (a)), B04 (a malformed `/ByteRange` is read as
  pairs), B05 (a numeric UR `/V` counts as understood), B06 (the DTS whole-file rule keys on the
  sub-filter), B07 (a DSS revision after a PAdES signature is reported as breaking Table 255, in
  `pades_departures` and in the report).
- Eight failing `*_spec_audit` tests, in `expected-failures.json` and in
  `2038-specification-audit/expected-failures-current.json`. The signed fixtures come from
  `crates/pdf-signature/tests/fixtures/spec_audit_2128/generate.py` with a fresh test key.

## Ledger

All 36 §12.8 rows are `implemented`. B findings contradict 12.8.1, 12.8.2.3, 12.8.3.3.1,
12.8.3.4.2, 12.8.3.4.3 and 12.8.5.2, and their aggregates. The fix round decides on them.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets` over `-p pdf-signature -p viewer-core`
exit 0 on the fourth run; the first three found lint shapes in the new tests. `pdf-sandbox --bins`
exit 0. nextest `--lib --bins --tests` of both crates: 1312 run, 1304 passed, 8 failed. The 8 are the
manifest's new entries, and the difference is 0 and 0. rustdoc `-D warnings --document-private-items`
exit 0. `cargo test -p conformance --no-fail-fast` exit 0 once this paragraph existed. No public type
changed, so fuzz is not owed.

## Habit

Audit what a signature binds, and not only what it says. Every answer in the crate was right about
the field it read. The defects are in the pairs of fields that no single answer compares.
