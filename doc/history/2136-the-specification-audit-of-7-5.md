# Round 2136 — the third specification audit: §7.5, file structure

An audit round in the `audit` worktree, made in the shape of review 2038 and audits 2128 and
2131, turned on where untrusted bytes first enter: §7.5.1 to §7.5.8.4, §7.3.10 and §7.3.8.2's
`/Length`, against `pdf_syntax`'s reader and both writers. It fixes nothing. No ledger row moves.

## What it found

- `doc/reviews/2136-specification-audit-7-5.md` and its directory. Eight findings: 5 B, 1 T, 2 D.
- B01 a compressed object is parsed past the next member's offset, so `57` stored as two
  members reads as `57` (NOTE 7's 2020 correction); B02 the update writer counts its offsets from
  byte zero, so an update to a file with bytes before `%PDF-` is rebuilt by scanning, a freed
  object comes back, and the file cannot be saved again; B03 a conforming 526-byte file of two
  million free entries costs the reader about 108 MiB, with nothing but `max_stream_len` and the
  1024-section cap above it; B04 `serialize` writes an object-stream index past 65 535 as 65 535;
  B05 the update writer writes an offset its field cannot hold (no test: it needs 4 GiB).
- T01 a hybrid section's own free entry beside its `/XRefStm` is unpinned; D01, D02 are prose.
- Four failing `*_spec_audit` tests in `crates/pdf-syntax/tests/file_structure_spec_audit.rs`,
  each with a control; listed in the audit's `expected-failures.json` and appended to
  `2038-specification-audit/expected-failures-current.json`.
- The brief's premise that §7.5.7 leaves an object stream's dictionary unencrypted did not hold.

## Ledger

Fifteen rows audited: 14 `implemented`, 7.5.5 `departed`, before and after. B findings
contradict 7.5.2, 7.5.4, 7.5.6, 7.5.7, 7.5.8.2 and 7.5.8.3, and the aggregates 7.5 and 7.5.8.

**Gates.** fmt exit 0. clippy `-D warnings --all-targets` over `-p pdf-syntax` exit 0 on the
second run; the first found an unfulfilled `clippy::panic` expectation in the new test file.
`pdf-sandbox --bins` exit 0. nextest `--lib --bins --tests` of `pdf-syntax` (`xvfb-run -a`): 410
run, 406 passed, 4 failed, 1 skipped, the manifest's 4 ("failed 4, expected 4"). rustdoc `-D
warnings --document-private-items` exit 0. `cargo test -p conformance --no-fail-fast` exit 0 on
the third run; the first two wanted this paragraph and a shorter record. Fuzz not owed.

## Habit

An offset has an origin on the way out as well as on the way in. ADR 2960 taught the reader to
count from the header; the writer that appends to the same file kept counting from byte zero.
When a reader learns where a number is measured from, grep the writers of that number.
