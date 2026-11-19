# 3109 — A Type 0 font naming no CIDFont is given one that draws nothing at the width this reader moved by

Session 2102, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s R10
entry for `issue12823`. Status: accepted and built. In `pdf-transform`:
`archive/fonts/no_descendant.rs` (new: `lacks_one`, `written`), `archive/fonts.rs` (`embed_pass`
takes the route ahead of every other for such a font; `SubstitutedFont::descendant_written`;
`empty_cids_given` words the case), `archive/decision/fonts.rs` (the row moves from
`REFUSED_BY_NAME` to `REMEDIES`, `Answer::Stated` over `Rewrite::SubstituteFontProgram`),
`archive/report.rs` and `archive/prepare/font_rewrites.rs` (reported and recorded). Tests:
`crates/pdf-transform/tests/archive/no_descendant.rs`
`a_type0_font_naming_no_cidfont_draws_and_moves_as_this_reader_did`,
`a_type0_font_naming_no_cidfont_over_another_cmap_stays_refused_by_name`; `tests/archive.rs`
`a_type0_font_whose_descendant_is_null_is_given_one_that_draws_nothing` (replacing
`..._is_refused_by_name`). Context: ADR 2433 (the row and its refusal), ADR 1094 (the font placed
by its widths alone), ADR 2824 (the empty glyph under a `/CIDToGIDMap`), `doc/questions/A48` (the
condition). Clauses: ISO 32000-2 §9.7.6.1 Table 119, §9.7.4.2, §9.7.4.3 Table 115, §9.4.4, §9.3.6;
ISO 19005-2 sections 6.2.11.2, 6.2.11.4.1, 6.2.11.5, 6.2.11.8; ISO 19005-4 sections 6.2.10.2,
6.2.10.4.1, 6.2.10.5, 6.2.10.9.

## Context

`issue12823`'s `/FT19` is an `Identity-H` Type 0 font whose `/DescendantFonts` is `[ null ]`. It
fails section 6.2.11.2's row alone, and was refused by name. The census's R10 entry (session 2097)
read that the window draws nothing for it **and does not move the text matrix**, and asked for a
content walk proving that no character spacing moves a later glyph before the remedy could write
a font that advances by `Tc`.

That reading was half right. `LoadedFont::load` refuses the font, and the interpreter then places
its codes by `LoadedFont::metrics_only` (ADR 1094): the `CMap` splits the string, and each code
moves the pen by Table 115's `/DW` default, "1000", plus §9.4.4's spacings — the document's two
shows are each followed by `ET`, which is why emptying the strings left page 1 byte-identical. So
the window draws nothing and advances by 1000 thousandths of an em a code.

## Decision

1. **The written `CIDFont` moves as the window moved.** A `CIDFontType2` over a program holding
   glyph 0 and one empty glyph (`for_embedding_with_empty_glyphs`, ADR 2824), that glyph's advance
   restated to the one width the window placed every shown code by (proved equal for each code
   before anything is written), `/DW` stating it, a `/CIDToGIDMap` stream sending every CID to the
   empty glyph, `Adobe-Identity-0`. §9.4.4's `w0` is then the window's, and `Tj`'s adjustment, `Tc`,
   `Tw` and `Th` are the graphics state's, applied alike on both paths. **So no content walk is
   owed**: the proof is the width, and it holds whatever spacing is in force.
2. **A clipping mode needs nothing either.** An empty glyph adds no outline, and §9.3.6: "If no
   glyphs are shown or if the only glyphs shown have no outlines ... no clipping shall occur."
3. **The route is held to the case it was proved for**: an object of its own, `Identity-H`
   (two-byte codes, horizontal, each code its own CID), strings the survey kept whole, a font the
   window draws no glyph of and places by its widths. Every other font that fails the row keeps a
   refusal naming those conditions.
4. **A48's condition.** The font states no character: a `CIDFontType2` reaches its glyph by index
   (§9.7.4.2), and the producer's `/ToUnicode` stays. The shown CIDs are listed in the report and
   in `xmpMM:History` with the clause (`descendant_written`). No loss is named: the output draws
   and places what the window did. The one difference is the text a reader extracts: the window's
   metrics-only font names no character, and the output's font carries the producer's
   `/ToUnicode`, which is the file's own statement and not this conversion's.
5. **`Answer::Stated`** over the substitution's rewrite, since the substitution's preparation is
   where a program is embedded and its refusal is photographable under the profiles that allow it.

## Consequences

`issue12823` converts at 2b and 4 under `as-shown` and `as-if-printed`; veraPDF 1.30.3 passes all
four outputs and page 1 at 150 and 300 dpi is byte-identical to the source's. The fixture with a
character spacing and a glyph of another font placed after the font's string draws pixel for pixel
as its source, and moves when the string is emptied, so the comparison would see a font that moved
otherwise. The survey records no spacing and no public type of `pdf-archive` changed;
`SubstitutedFont` gains one public field.
