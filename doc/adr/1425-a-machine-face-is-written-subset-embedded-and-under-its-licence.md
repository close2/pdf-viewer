# 1425 — A machine face is written into a file subset, embedded, and under its licence

Session 1294. Status: **accepted** and built.
Context: `crates/pdf-font/src/embed.rs` (new), `crates/pdf-model/src/variable_text.rs` (`for_a_file`),
`crates/pdf-model/src/appearance.rs` (`for_annotation`, `for_saving`), `crates/pdf-model/src/view.rs`
(the three save sites, `promote_resource_streams`), `crates/pdf-transform/src/archive/prepare.rs`
(`streams_numbered`), `crates/pdf-model/tests/machine_face_written.rs` (new).
Amends: ADR 1414 section 4 ("A machine face is drawn, not written"), which it replaces.
Clauses: ISO 32000-2 §9.9.1, Table 124, Table 125, §9.9.2, §9.7.4.2, Table 115, Table 119, §9.8.2,
§7.3.8.1, §12.5.2, §12.7.4.3.

## 1. What ADR 1414 left owed, and what it did not name

A value no compiled-in face can draw is set in a face the machine offers, as a `Type0` font over a
`CIDFontType2` whose program is the whole face held as a direct stream. ADR 1414 refused to write it
for two reasons: §7.3.8.1 ("All streams shall be indirect objects"), and a face not subset whose
licence was never asked. Reading §9.9.1 found a third it did not name: "If used with a CIDFont
dictionary, the "cmap" table is not needed and shall not be present" — the whole face carries one.
(The brief for this round asked for `cmap` in the subset; the clause says the opposite, and the
clause is what was built.)

## 2. The construction

- **The drawing is unchanged**; the file's form is derived at the save sites by
  `variable_text::for_a_file`, so every screen path stays as ADR 1414 measured it.
- **Subset** (`pdf_font::embed::for_embedding`): the glyphs `/W` states, glyph 0 (§9.9.2's
  `.notdef`), and every component a kept composite names, to any depth; `glyf` rebuilt with long
  `loca` offsets and component indices rewritten, `hmtx`/`hhea`/`maxp` restated, `cvt `, `fpgm`,
  `prep` carried where present (the table's "if they are required by the font instructions" is not
  something this reads, so they are kept), `OS/2` carried so the licence travels, and nothing else.
- **Names**: `/BaseFont` of both dictionaries and `/FontName` are `TAG+PostScriptName`; the tag is
  an FNV-1a digest of the program and the kept glyphs, so different subsets differ and a save is
  deterministic. A collision with a producer's own tag in the same file is possible at one in 26^6
  and is accepted.
- **CIDs are not renumbered.** The content stream keeps the face's glyph indices as CIDs, so the
  stream written is the stream drawn; `/CIDToGIDMap` is Table 115's stream wherever the subset
  renumbered, `/Identity` where it did not. Table 125's `/Length1`; both streams under `FlateDecode`.
- **Table 121's flag is `Symbolic`** in both forms: §9.8.2 requires it for glyphs outside the
  Standard Latin set. The drawn font said `Nonsymbolic` on a reading that belongs to simple fonts.
- **Streams become objects where they are written**: `view::promote_resource_streams` in the
  incremental update (ADR 0100), `prepare::streams_numbered` in the archive conversion.

## 3. The licence, and why no reader's level reaches it

The OpenType specification's `OS/2` `fsType`: restricted-licence usage (with neither of the two
permitting bits) and bitmap-embedding-only refuse the write; preview-and-print and editable permit
it, the least restrictive bit set winning as the specification has a reader do for early table
versions; no-subsetting keeps every glyph at its own index under the plain PostScript name. A face
with no `OS/2` table states nothing and is read as installable — a choice, written in
`embed::permission`. §9.9.1 is the PDF side: "One of the conditions may be that the font program
cannot be embedded, in which case it should not be incorporated into a PDF file", and "embedded font
programs shall be used only to view and print the document", which is what a preview-and-print face
permits. This is the font vendor's condition on this program, not a restriction a document asserts
over its reader, so CLAUDE.md's four levels do not apply: a refused face leaves the appearance owed
(`Written::unconstructed`, `Written::unappeared`, `Written::owed`), with the sentence.

## 4. Measured

`an_arabic_value_saved_in_a_machine_face_draws_from_the_file_alone`: a field's Arabic value saved,
`qpdf --check` clean, the machine's faces then turned off for the process, the file re-opened and
drawn — byte-identical to the on-screen raster at 2x. Planting `/CIDToGIDMap /Identity` for the
renumbered subset fails it. `pdf_font::embed`'s four unit tests (composite closure and renumbering,
tags, licence bits, no-subsetting), and `a_face_whose_licence_forbids_embedding_stays_owed` on this
machine's own face with `fsType` restated to 0x0002. Where the machine offers no Arabic face the
tests skip with ADR 1154's sentence.
