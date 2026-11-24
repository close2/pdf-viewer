# Round 2225 — the site's 182 are read, and two of them change the tree

A clause round in `lane3`. It read the 182 quotations that round 2222's `spec-errata check` found
on text only pdf-issues.pdfa.org strikes, using `doc/errata-read.md`'s method. Sonnet helpers
fanned the reading out by clause family. This round read for itself every verdict that changes
code.

- The 182 come from 62 issues. All 62 are published on the site, so all are agreed (ADR 3581);
  twelve carry no `data-iso`. 70 of the 182 are in records (`doc/adr/`, `doc/history/`,
  `doc/reviews/` and `doc/errata-read.md`), which keep the words they quoted. The other 112 are
  restated: the verbatim untouched part stays, and the corrected words follow in italics, naming
  the issue (ADR 3603). That covers 54 Rust files across ten crates, plus `tools/spec-errata`,
  28 ledger notes, `doc/todo/11`, `doc/todo/30` and RFC 0015.
- ADR 3601: #850 (the owner's issue, agreed by the PDF TWG on 2026-11-05) removes `ca` and `CA`
  from §12.5.5's compositing sentence, so the row goes from `departed` to `implemented`. No code
  changes; ledger departed 11 → 10, implemented 787 → 788.
- ADR 3602: #784 (decided by the C&P TWG) limits §7.5.8.2's exemption to strings under keys that
  Tables 5, 15 and 17 define. `encryption::is_defined_cross_reference_key` applies the limit on
  the decrypt and the re-encrypt path. The test
  `a_cross_reference_stream_dictionarys_private_string_is_decrypted` fails without the change.
  The §7.5.8.2 row names the code and the test.
- #839 deletes the last bullet under Table 66. `pdf-archive`'s ICC-edition argument now rests on
  Table 66 as #181 replaces it; no predicate moved.
- Owed: #226's owner-scoped precedence, read against `structure::attributes::ranked`. Also the
  struck words of ten record-only issues, which are still quoted in `check`'s other-clause
  population. `doc/errata-read.md`'s new section lists both.
- `check` afterwards prints 71 site-only lines on cited clauses: the 70 records and one false
  match (`form.rs`, Table 228 bit 13, against #392's capitalised *The*).

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0 (`pdf-archive pdf-colour pdf-font
pdf-model pdf-render pdf-syntax pdf-transform render-raster viewer-core viewer-host spec-errata
viewer-qt`); `pdf-sandbox --bins` 0; nextest `--lib --bins --tests` 0, 7512 of 7512, none in the
manifest, which is empty; rustdoc 0; `cargo test -p conformance` 0 once this paragraph was written.
Fuzz not owed: no public type changed. No golden or corpus check: no pixel can move (#784 changes
only strings under private keys of a cross-reference stream).
