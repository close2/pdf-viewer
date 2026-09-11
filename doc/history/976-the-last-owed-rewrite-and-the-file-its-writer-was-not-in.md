# 976 — The last owed rewrite, and the file its writer was not in

Date: 2026-09-11. ADR: 0988.
Files: `crates/pdf-font/src/restate.rs`,
`crates/pdf-transform/src/archive/{fonts,decision,prepare,rewrite,report,mod}.rs`,
`crates/pdf-transform/tests/archive.rs`, `doc/pdf-a-mitigations.md` §5, §13.3, §13.3.1,
`doc/pdf-a-conversion-limits.md` §4.9, `doc/adr/0988`, `doc/history/976`.

The PDF/A converter stream, continuing sessions 962, 966 and 971. **The twenty-second and last of
`doc/pdf-a-mitigations.md` §13.3's owed rewrites is built** —
`fonts/vertical-metrics-agree-with-the-program`, ISO 19005-4 6.2.10.5's third paragraph — and the
list is now empty.

What is worth carrying:

- **§9.9.1 settles the writing direction outright, and its *previous* sentence answers a question
  three readings of this row never asked.** The vertical sentence forbids a PDF processor from ever
  using `vhea` and `vmtx`, which makes restating them unobservable by construction. The paragraph
  before it lists the TrueType tables that "shall always be present if present in the original
  TrueType font program" and does **not** list those two — so *removing* them is as lossless as
  restating them, and this row had the `/CIDSet` pair's two routes all along. Restating is what was
  built, for the opposite of the usual reason: removal is the **harder** build, because a directory
  cannot lose a record without being rebuilt.
- **Naming a module is not naming a writer, and a catalogue entry that names where work belongs is
  a claim about the tree that decays.** The entry said the row waited on `pdf_font::restate`, which
  "rewrites an sfnt's `hmtx`". True of the module's subject, false of its code: `restate.rs` owns
  the units, the tolerance and the format dispatch, and the splice, the directory update and the
  checksums are `sfnt.rs`'s — a file this round did not hold. Three rounds were blocked on the
  wrong file name.
- **The constraint produced the better rewrite.** The horizontal restatement has to lengthen
  `hmtx`; the vertical one does not, because every advance it writes already has a field. So
  `with_vertical_advances` overwrites `uint16`s where they sit — no offset, no length, no outline
  moves — and refuses by name the one case that would need the table lengthened: a glyph in the
  tail past `numOfLongVerMetrics`, whose advance would be restated along with every other glyph
  there, none of which was asked for.
- **Two checksums adjusted rather than recomputed, and it is a claim, not a shortcut.** One
  `uint16` at an even offset in a four-byte-aligned table is one word; the table's sum moves by
  that word's difference and the file's by twice it, the record's `checkSum` being itself a word of
  the file. A test recomputes both independently and compares. A program whose producer's checksums
  were already wrong keeps exactly the error it arrived with — `doc/adr/0947`'s first rule at the
  level of a byte.
- **A rewrite that reaches an object another rewrite already replaces is not a competitor.** Both
  metric restatements replace the same font program stream. One replacement map per rewrite makes
  "restated in both directions" inexpressible and drops the second write in silence; one
  replacement per object, plus a set per requirement saying which asked, expresses it. They stay
  separate `Rewrite`s all the same, because part 2 states no vertical rule and `wanted_by` has to
  be able to say so.
- **A silent clamp next door.** `with_widths` turned an advance past 65 535 design units into
  **zero** with `unwrap_or(0)`. Now `RestateError::OutOfRange`. It was found by writing the same
  conversion a second time and asking what its upper bound was.
- **The corpus said nothing, and could not have.** The veraPDF figures are unchanged at all six
  targets: no document in it sets a composite font vertically with a disagreeing `vmtx`. This is a
  coverage question whose denominator is the specification, and a round waiting for the corpus to
  rank it would have waited forever.
