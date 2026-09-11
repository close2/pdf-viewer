# 962 — Four owed rewrites, and five that were not owed at all

Date: 2026-09-11. ADR: 0965.
Files: `crates/pdf-transform/src/archive/{decision,prepare,rewrite,sites}.rs`,
`crates/pdf-transform/tests/archive.rs`, `doc/pdf-a-mitigations.md` §4.1, §4.2, §4.5, §5, §7, §9,
§13.3 and §13.3.1, `doc/adr/0965`.

The PDF/A converter stream, continuing session 957 over the eleven of `doc/pdf-a-mitigations.md`
§13.3's twenty-two that were left.

**Four built**, each out of `REFUSED_BY_NAME`: the deprecated `bytes` and `encoding` attributes cut
out of an XMP packet header; the catalog's `/NeedsRendering` removed; and the two TrueType encoding
rows — a symbolic font's `/Encoding` taken away, a non-symbolic one given `WinAnsiEncoding` or
`MacRomanEncoding`.

**Five were not lossless rewrites after all** and are now refused with an argument instead of a
debt. **Two remain owed.**

What is worth carrying:

- **The proof the two TrueType rows needed is "load the font twice".** `LoadedFont` takes a
  dictionary and a document, so the candidate is a clone of the font dictionary with the entry
  written or removed, loaded against the same document, and `glyph_index` compared code by code.
  Nothing re-reads §9.6.5.4 — the procedure is the font reader's, asked twice.
- **And its denominator decides whether it ever fires.** Over the codes the content streams
  *showed* — this tree's precedent in `archive::fonts` and `archive::to_unicode`, and what the
  catalogue asked for — the rewrite works. Over all 256, `StandardEncoding` and `WinAnsiEncoding`
  reach different glyphs at **80** of them in a full Latin face, so the rewrite would never once
  have fired and would have been indistinguishable from not existing.
- **A rewrite can be declared and never written.** `Rewrite::PacketHeaderAttributes` was added in
  session 957 with a doc comment, a `describe` arm and a `word` arm, answered no `REMEDIES` row,
  and named a `pdf_model::xmp` function that did not exist. An unused enum variant is not a compile
  error, which is how it crossed a round.
- **The vertical metrics entry had the writing direction backwards and would have moved marks.**
  §9.2.4 makes the font dictionary's numbers what positions a glyph, and §9.7.4.3 gives `/DW2` and
  `/W2` that role going down the page; restating them from the program moves every glyph on a
  vertical line. It also named the wrong half of the tree as the debt — the reader already states
  the program's vertical advance and the *writer* does not write one.
- **"Not built yet" and "not decided" are different facts, and the catalogue had filed the second
  under the first.** The duplicate-profile pair is reported at a page with no object and would
  resolve an overprint ambiguity §8.6.7 leaves open; the JPEG 2000 pair's cost is a hundred bytes
  and its *value* is a choice between two of the producer's own statements; and a `Separation`'s
  one-input tint transform cannot be composed out of a `DeviceN`'s *N*-input one, because §7.10
  gives a PDF function no way to call another.

ADR 0965 has all of it. The census still reports **0** requirements with no considered answer at
any of the six targets, and `archive_unconsidered.txt` is still empty and still held to equality in
both directions.
