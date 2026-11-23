# Round 2201 — the repair channel, and section 5.1 checked over it

The first fix round of the silent-repair campaign (ADR 3477, review 2194), in the `gwg` worktree.

## What changed

- `pdf_syntax::repair` (ADR 3505): `RepairClass` (stable name, clause, what was read instead),
  `Site` (`WholeFile`, `Offset`, `Object`, `Within`), `Repair`, a bounded `RepairLog`, and 41 classes in
  `CLASSES`. `Document::record_repair`, `record_repairs` and `repairs` are the one channel; the
  `Lexer`/`Parser` and the `XrefTable` carry their own until a document takes them.
- Every inventory site of the lexer, parser, object loading, object streams, cross-reference
  reader, decoding and encryption logs its class or carries `// not a repair:` with its clause;
  the existing reports (`misfiled`, `lost_to_damage`, `padded`, `Damage`, the chain cut) stay.
  `filter.rs`'s sites were sorted by a helper: 11 reach the log through `Damage` or a refusal, 2
  are the clause's own reading, 5 are silent and left (ADR 3505 names them).
- `ContentReader::lexical_repairs` keeps the classes lexing a content stream needed; the PDF/A
  survey records them against the page or form.
- `pdf_archive::repairs` (ADR 3506): `MAPPING` maps each class to section 5.1 and its other rows
  (6.1.13 integers, 6.1.9 object syntax, 6.1.3 encryption) or to no row by name (two bounds of
  this reader). Section 5.1's row is `Implemented`; a log-reading row reads the whole file first.
- The converter answers 5.1 as one the writer satisfies (`WRITER_EMITS`).
- 2194-B01, B02, B03, B04, B06, T01, T02, T06 fixed; their nine tests pass and leave
  `expected-failures-current.json`.

## Trap

A run of digits too long for the lexer's fixed format reaches `f64::from_str` like an exponent
does. The first cut called both "exponential", and every converted fixture whose serializer
wrote a long real failed its own section 5.1: a class must be tested on the converter's output,
which is the one population guaranteed to be well formed.

**Census.** With and without the pack the same 18 documents (5 pdf.js, 13 corpora) stop converting:
the output fails section 5.1, because what the rewrite carries byte for byte — a damaged stream, a
content stream with an unterminated string or a stray delimiter — is still in its log. No remedy is
built for either; the converter should refuse them by name (left).

**Gates.** fmt 0; clippy `-D warnings --all-targets` 0; `pdf-sandbox --bins` 0; nextest of the six
crates 100, 5601 of 5610 passing, the nine failures exactly the manifest's (rounds 2 to 4's);
rustdoc 0 on each; fuzz `cargo check --bins` 0; `cargo test -p conformance` 0 once `Site::File`
became `WholeFile` (the names gate read the variant as a declared `File`).
