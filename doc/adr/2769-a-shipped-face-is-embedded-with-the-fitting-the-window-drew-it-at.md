# 2769 — A shipped face is embedded with the fitting the window drew it at

Session 1979, a single archive round.
Status: accepted and built. In `pdf-font`: `stretch.rs` (`sfnt_outlines`, `within_in_ems`) and its
tests in `stretch_tests.rs`. In `pdf-transform`: `archive/fonts/described.rs`
(`with_the_fitting`, `fitting_kept`, `same_once_written`, `same_glyph`), `archive/fonts.rs`
(`Routes::write_fitting`, `Fitting`), `archive/fonts/truetype_cmap.rs` (the restatement's proof
accepts the face the same writer scaled), the report and the file's history.
Tests: `stretch::tests` (five) and `crates/pdf-transform/tests/archive.rs`'s
`a_shipped_face_fitted_horizontally_is_embedded_with_its_fitting` and
`…_is_the_loss_substitute_stretch_where_authorised`, and
`tests/archive/truetype_cmap.rs`'s `…_restated_over_the_face_carrying_its_fitting`.
Context: ADR 0358 (the window's fitting), ADR 2262 and `doc/questions/A566` (the loss
`substitute-stretch`), ADR 2490 and `doc/pdfa-blocking-issues.md` R09 (b), ADR 0988 (prove before
writing), owner answer A550 (a modified Liberation face is renamed). ADR 2770 says which profile
takes it.
Clauses: ISO 19005-2 sections 6.2.11.4.1 and 6.2.11.5, ISO 19005-4 sections 6.2.10.4.1 and
6.2.10.5; ISO 32000-2 §9.2.4, §9.6.2.1 (Table 109's `/Widths`), §9.9 (Table 124); ISO/IEC 14496-22's
`glyf`, `loca`, `head`, `hhea` and `hmtx`.

## Context

Both parts require every font a page renders to be embedded. Where a file names a font it does not
embed, this reader draws a face it ships, and where the file's `/Widths` say the absent font was
narrower, it scales the face's outlines in x by one number, the median ratio of stated width to the
face's advance, about each glyph's origin (ADR 0358). Embedding the face unmodified removes that
scale. That is the loss `substitute-stretch` (ADR 2262), which `as-shown` withholds, so `as-shown`
stopped on every such font. The census put the loss alone on 32 documents at both parts, and on 6
more at one part. Every one of the 38 is a Liberation Sans face, at scales from 0.9928 to 0.99999;
the smallest scales come from integer `/Widths` against a 2048-unit em.

## Decision

1. **The scale is written into the face's outlines, and nothing else moves.** Each simple glyph's
   x coordinates become `round(x × scale)`, re-encoded with the x bits of each flag restated (no
   repeat counts) and the y bytes copied. Each composite's x offset is scaled, and a two-by-two
   is conjugated (`S·T·S⁻¹`: the term adding y into x times the scale, the term adding x into y
   divided by it). Point-matching arguments are kept. Each glyph's box, `head`'s box, `hhea`'s
   three horizontal extremes and each left side bearing follow, the bearing chosen so that the
   first phantom point stays at the scaled origin. `loca` is written long. The advances, the
   instructions and every other table are copied byte for byte. §9.2.4 positions by the
   dictionary's widths, and section 6.2.11.5 holds the program's advances to them; the existing
   restatement (`restate::with_widths`) runs after the scaling, exactly as for an unscaled face.
2. **Proved before it is written.** `sfnt_outlines` reads every glyph of the rewritten program
   back through `read-fonts` against the original. It requires the same points, each y equal, each
   x within one unit of the em of `x × scale`, and every advance equal. The conversion then loads
   the converted font from a written file and holds each shown glyph to the window's outline within
   that unit (`same_once_written`). One unit is the rounding to the program's integer grid: half
   for the point, half for a component's offset. For Liberation that is 1/2048 em, 0.006 pt at
   12 pt.
3. **The program is the font's own and is renamed.** A modified Liberation face loses its reserved
   name (A550). The program is not shared with other fonts, since each font's scale is its own.
   The report and the file's `xmpMM:History` state the scale written.
4. **Refused, falling back to the loss by name**: a scale outside ADR 0358's `[0.5, 1)`, a face
   that is not a `glyf` sfnt (the Foxit faces are bare CFF; none of the 38 uses one), a variable
   font, a number that no longer fits its field, and a read-back that disagrees. The face then goes
   in unmodified and the font names `substitute-stretch`, as before.

The licences allow it. Liberation is SIL OFL 1.1, whose condition 3 is met by the rename this tree
already performs for a restated face (`doc/third-party-data.md`). The Foxit faces are BSD-3-Clause.

## Consequences

The 32 documents R09 (b) holds alone convert under `as-shown` at both parts, and six more at the
part their other refusals leave open. What is left of R09 is (a), `machine-face`, and a fitted CFF
face, which would take a `FontMatrix` rather than an outline rewrite and has no witness yet.
