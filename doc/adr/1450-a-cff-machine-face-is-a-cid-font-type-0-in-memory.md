# 1450 — A CFF machine face is a `CIDFontType0` in memory too

Session 1307. Status: **accepted** and built.
Context: `crates/pdf-model/src/variable_text.rs` (`machine_font`, `machine_font_for_a_file`),
`crates/pdf-font/src/embed.rs` (`has_cff_outlines`, `glyph_indices_reach`).
Amends: ADR 1414's drawing path, which described every machine face as a `CIDFontType2`.
Clauses: ISO 32000-2 §9.7.4.2, Table 117, Table 124, Table 125.

## Decision

`machine_font` describes the face by the `CIDFont` Table 124 puts its outlines under. A `glyf` face is
a `CIDFontType2` over `/FontFile2` with `/CIDToGIDMap /Identity`, as before. A `CFF ` face is a
`CIDFontType0` over `/FontFile3 /Subtype /OpenType` with no `/CIDToGIDMap`, because Table 117 states
that entry for Type 2 only. The loader already chose its route by the program's bytes, so this changes
the description and not the drawing. `a_cff_machine_face_is_a_cid_font_type_0_in_memory_and_draws_as_before`
loads both descriptions of a machine face and compares every glyph index and outline, and they are
identical. The fixture's screen raster hashes to the same value before and after the change
(`a55f473ca05ea843`, 400x200).

Under a `CIDFontType0`, §9.7.4.2 reaches a CID-keyed program's glyphs through its charset. A CID-keyed
face whose charset gives a displayed glyph a CID other than its index would draw another glyph, and the
old `CIDFontType2` description would have drawn that same other glyph, because the loader reads the
program either way. So `machine_font` answers `None` for such a face (`glyph_indices_reach`), and the
value keeps the refusal it already had. No face on this machine is CID-keyed. Showing the charset's
CIDs in the stream instead is the construction that would admit such a face, and it is not built.
