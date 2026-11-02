# 2614 — A glyph's walk from a string's byte to three rasterisers is the sixth teaching document

Session 1890, slot 10 of batch 129, a docs round. Status: **accepted** and **built** in
`doc/teaching/how-a-glyph-reaches-the-screen.md` and `tools/conformance/tests/teaching.rs`.
Context: the owner's code-quality slot; ADR 2002 (the first walk and the diagram convention), ADR
2003 (what the gate holds and what a note holds), ADRs 2062, 2078, 2554 and 2574 (the second to
fifth walks); ADR 0133 (the standard 14 are this tree's faces), ADR 0152 (a code that reaches no
glyph stays visible to the instruments), ADR 2146 (published widths for a font named alone), ADR
2572 (the operation budget), ADR 2586 (a refusal kept and reported), ADR 2570 (the dot at a dash's
end). Clauses: ISO 32000-2 §9.2.4, §9.3.6, §9.4.4, §9.6.5.2, §9.7.4.3, §9.7.6.2, §9.7.6.3 and
§8.4.3.6. ADR 2615 and Q766 are unused.

## 1. The gap

The first walk's stage 3 hands a `Tj` to `pdf-font` in one sentence, and the walk from a word to a
clipboard reads the same codes for their text, not their shape. Between them sit the decisions a
student meets first in a PDF renderer and finds hardest to locate: where a code becomes a glyph,
which program is run and under what bound, where the outline is kept, what positions it, and what
each backend does with it. The module comments argue each one; none says what happens in order.

## 2. Decisions

1. **Drawn at one commit, and said so.** The walk describes commit c53ad5e0. Two sibling rounds of
   the same batch changed `crates/pdf-font/src/cff.rs` in the shared tree; the walk names nothing
   either added, every claim was checked against `git show HEAD:<path>`, and its names were held by
   the gate both in the shared tree and in an export of HEAD.
2. **Five stages, in the order the code takes them**: selection (`loading/codes.rs`, `cmap.rs`,
   `loading/glyphs.rs`, the code table built at load); shape (`loading/drawing.rs::build_outline`,
   `cff.rs`, `type1.rs`, `skrifa`, the budget being `read-fonts`' own); memory and honesty (the
   outline cache and `account_for_a_missing_mark` to `complete`); place (`loading/advances.rs`,
   `metrics.rs::simple_widths`, `placement.rs`); pixels (`glyph_painting.rs`, `paint.rs`, the
   display list, the three backends and `dash_end.rs`). The brief's hypothesis — five stages, six
   diagrams — held.
3. **The running example is a standard 14 font named alone**, `Times-Roman` with no `/Widths` and
   no program, because it alone touches every stage: a code table built against a face this tree
   carries, the published metrics as the width source, and with `1 Tr` on the second line both a
   fill and a stroke. A composite or embedded font is described where it leaves the route.
4. **The width sources are drawn per font, not per code**, because that is what `simple_widths`
   does: a non-empty `/Widths` is the whole table and a code outside it takes `/MissingWidth`; the
   published metrics and the substitute's own advances apply only to a substitute stating none.
5. **The example's figures were read off a build**: the page written out as a file, drawn with
   `render_at` and read back with `readback` from a `dev` build of an export of HEAD; the pen
   positions are derived from the published widths and say so.
6. **The gate holds six walks from one table.** `WALKS` gains the entry with six diagrams; the
   calibration is unchanged. A renamed path planted in the new walk was reported by name and then
   taken out.

## 3. Cost

About five hundred lines of prose held by the gate only for names, over `pdf-font`, the crate two
other rounds of the same batch were changing. The maintenance notes are the price: each names its
files and the change that redraws its diagram, and stage 2's names the budget's owner as the thing
to keep true.
