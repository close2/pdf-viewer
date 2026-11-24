# 3541 — `pdf-model` logs its repairs on the one channel: the page tree, the content reader, the interpreter

Session 2210, the silent-repair campaign's second fix round. Status: **accepted**. Context: ADR 3477
(the decision), ADR 3505 (the channel as built), ADR 3521 (the content site); review 2194's
`pdf-model` inventory, B07 and B08; ISO 32000-2 §7.7.3, §7.8.2, §8.4.2, §8.6.8, §8.9.7, §9.4.1,
§14.11.2.1.

## Context

Round 2201 built the channel and the syntax layer's classes. `pdf-model` repairs at a level the
syntax layer cannot name — a `/MediaBox` that is a name, a `/Rotate` of 45, a `Q` with nothing
saved — and its reports (`substituted_media_box`, `Unsupported`, `ContentIssue`) reached neither the
validator nor, for most of them, the viewer.

## Decision

- **`pdf_model::repair::CLASSES`**, 25 classes declared as `pdf_syntax::repair` declares its own
  (`model/…`, a clause, what was read instead), recorded on the document the reader reads from.
- **The page tree** logs at `Site::Object` of the node or page that stated the entry: the catalog,
  an unusable root `/Count`, a tree replaced by a scan, a damaged page dictionary, a `/Kids` element
  that is no node, a cycle, a bound, `/Resources` absent or not a dictionary, a substituted
  `/MediaBox`, an optional boundary that is no rectangle with an area, a `/Rotate` that is no
  integer or no multiple of 90, a `/UserUnit` that is no positive number. A box outside the medium
  is not a class: §14.11.2.1 itself makes it its intersection. A unit past 1000 is not either: Table
  31 leaves the range to the implementation.
- **The content reader** logs at `Site::Content` of the page or form: `ContentReader::for_page`
  holds its document and records, as it is dropped and as its issues are taken, the lexer's classes,
  the inline objects' (`note_repair`), a part that is not a stream and a bound it reached. A
  damaged or undecodable part is left to the decoder, which logs `pdf_syntax`'s own classes and
  tells the file's damage from a bound of its own (ADR 3505); the census's `bomb_giant.pdf` is a
  bound the reader's `Undecodable` would have called damage. The inline
  image reader returns its classes in `Scan::repairs` for the caller holding the reader.
- **The interpreter** logs where it already reported or silently repaired: a shortfall, a wrong-typed
  text operand, a `Q` with nothing saved, a text operator outside `BT`, colour operands read in the
  device space of their count. Its other tolerances (review 2194's T05) are round 4's.
- **Bounds of this reader** (`page-tree-past-a-bound`, `content-past-a-bound`) are classes, mapped
  to no row by name in `pdf-archive`.

## Consequences

- A well-formed page logs nothing; the push is on the error path only.
- The inventory's outline, structure tree, XMP and JPX sites (M146–M187) stay unlogged: they are
  off this round's three readers and are named as left in the record.
