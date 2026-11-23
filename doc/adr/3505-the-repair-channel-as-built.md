# 3505 — The repair channel as built: a site, a lexer that carries its own, and classes by family

Session 2201, the first fix round of the silent-repair campaign. Status: **accepted**. Context:
ADR 3477 (the decision and its shape), review 2194 (the inventory), ISO 32000-2 clause 7.

## Context

ADR 3477 fixed the shape: `pdf_syntax::repair::Repair { class: &'static RepairClass, at: Location }`,
a log on the `Document` beside the object cache, one `CLASSES` slice per crate, and every existing
report also logging its class. Building it showed three places where the shape needed a decision
the ADR did not make.

## Decision

1. **`Site`, not `Location`.** `pdf_syntax::Location` is already a public root export naming a
   cross-reference entry's place (`Offset` or `InStream`), and a second `Location` one module away
   would make every import ambiguous. The repair's place is `repair::Site`: `WholeFile`, `Offset(n)` (a
   byte of the file), `Object(id)`, and `Within(id, n)` (a byte of an object's *data*, a decoded
   object stream or content stream). The whole file is `WholeFile` rather than `File`: the
   workspace's names gate reads a variant as a declared name, and a `File` of ours would make every
   doc comment's `File::open` a path into this enum.
2. **Two readers below the document carry their own repairs.** The `Lexer` (and the `Parser` over
   it) and the `XrefTable` run before or without a document. The lexer keeps a small vector of
   repairs at offsets into its own input, empty and unallocated wherever the input is well
   formed; the caller that knows what the bytes are records them on the document with
   `Document::record_repairs`, which moves each offset to its place (`Repair::placed`). The table
   keeps a `RepairLog` the document copies when it is built, and a rebuilt table inherits the
   log of the table it replaced. The content reader in `pdf-model` keeps the lexer's *classes*
   (`ContentReader::lexical_repairs`), because a windowed reader's offsets are not offsets into one
   buffer; the PDF/A survey records them against the page or form `XObject`.
3. **Classes by family, not by line.** 41 classes cover review 2194's `pdf-syntax` sites: one class
   is a kind of departure the file makes (`syntax/stream-extent-from-endstream` covers the parser's
   direct `/Length` and the document's indirect one), so a validator maps a kind once. A site whose
   reading is the specification's own carries `// not a repair:` and its clause.
4. **The log is bounded and deduplicated**: at most `SITES_KEPT_PER_CLASS` (4096) sites a class,
   the rest counted, so a stream of a million unmatched delimiters costs what a log keeps.
5. **A decoded stream's damage is logged at `Site::WholeFile`.** The decoder is handed a `Stream` and
   not its object, so the class says that a stream's data was cut short; the caller holding the
   object still has `Decoded::damage` to say which.

## Consequences

- Every repair recorded is recorded when it is made, and objects are read when asked for: a
  caller that wants the whole file's log reads the whole file first (`pdf-archive`'s
  `Examination::read_whole`, ADR 3506).
- The cost on a well-formed file is one length test per content token and one empty vector per
  lexer; the push itself is `#[cold]`.
- Left for later rounds, by name: `file.rs`'s short reads, `version.rs`, the name and number trees,
  text strings (pure functions with no document to record on), five filter tolerances that neither
  carry a `Damage` nor refuse (the PNG and TIFF predictors' short last row, Flate's leading white
  space and missing final block, ASCII85's `<~`), and the writers' tolerances, which are off the
  reader's path.
