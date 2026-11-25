# 3621 — A repair made without a document is returned to the reader that holds one

Session 2230, the silent-repair campaign's census burn-down, second pass. Status: **accepted**.
Context: ADR 3477, ADR 3521, ADR 3561, ADR 3565, ADR 3574 (the census and its `HELD` ratchet), ADR
3597; ISO 32000-2 §7.5.2, §7.8.3, §7.9.2.2, §9.10.3, §11.6.3, §12.7.8; `doc/questions/Q1051`.

## Context

Round 2224 left 571 held sites. The first of them were repairs made where no document is in hand:
`sfnt::horizontal_metric`'s truncated `hmtx`, the text-string decoder's U+FFFD, a `/ToUnicode`
destination's unpaired surrogate, `filter::decode`'s dropped damage; then `xref.rs`, `forms_data`,
32 sites of `appearance.rs` and an unlisted blend-mode name.

## Decision

- **A decoder returns its repair; the reader that holds the document logs it.**
  `pdf_syntax::text_string_noting` returns `syntax/text-string-undecodable` (§7.9.2.2) beside the
  text, and `Document::text_string(bytes, at)` decodes and records it on a `#[cold]` branch. The
  readers of an outline's `/Title`, a page label's `/P`, an optional content group's `/Name`, an
  annotation's and a structure element's text entries and a media object's text read through it;
  the other callers of `text_string` are the remainder, converted as their files are reached.
- **A `/ToUnicode` destination with an unpaired surrogate stays held.** Marking the map damaged
  (§9.10.3) moved nine documents of the census from PDF/A-2b to PDF/A-4, all through one producer
  pattern — an identity map over the surrogate block — so the reading is the owner's to settle
  (`doc/questions/Q1051`), with a remedy proposed.
- **Where the caller declines, nothing is repaired.** `horizontal_metric` answers `None` past the
  end of `hmtx`; its callers are writers and converters whose own documents already say "a font
  this cannot restate", so they decline (`StretchError::Unreadable("hmtx")`, `Refusal::Malformed`)
  instead of writing an advance of zero. `filter::decode` and `decode_with_parms` answer only with
  a whole stream, as their documentation said; a damaged prefix is `decode_reported`'s.
- **New classes**, each mapped: `syntax/header-missing` (§7.5.2; also section 6.1.2's
  `file-structure/file-header`), logged where no header stands anywhere and offsets are counted
  from the first byte; `syntax/text-string-undecodable`; `model/blend-mode-unknown` (§11.6.3:
  a single `/BM` name outside Tables 134 and 135, logged at the content site by the graphics
  state and the annotation readers, beside the report they already made);
  `model/resource-category-not-a-dictionary` (§7.8.3, Table 34). `appearance.rs` logs its
  annotation and field entries as ADR 3597's `annotation-entry-unusable` and
  `field-entry-unusable`, and a stored appearance's unusable `/BBox` as `form-bbox-unusable`.
- **An FDF file's repairs are said in its import's `owed`**, the report the person importing it
  reads: a flag entry of another type, an `/Opt` element that does not read, a `/Templates` absent
  or not an array (Table 251 requires it). The FDF is not the document a validator judges.
- **The converter's answers** follow ADR 3521 and 3597: the classes outside content are carried by
  the whole-file rewrite as the file states them and left to the output's own check, which refuses
  by name, except `header-missing`, which the rewrite answers by writing a header;
  `blend-mode-unknown` is at the content site and goes the way of ADR 3597's T05 classes.

## Consequences

- `HELD` falls from 571 to 515. `appearance.rs`'s last held site is `with_stand_in_font`, which
  holds no document; the remaining `text_string` callers are owed the same conversion.
- A conversion that wrote a zero advance for a glyph past a short `hmtx` now declines that route:
  pdf.js `bug911034` (an `hmtx` of 33 280 bytes where its counts need 117 394) is refused by name
  under `fonts/widths-agree-with-the-program`, whose sentence still speaks of a Type 1 charstring.
  Every pair it states is present and only the bearings after them are missing, so its advances
  are known; a bearing taken from the glyph's own `glyf` box would answer it without a guess.
