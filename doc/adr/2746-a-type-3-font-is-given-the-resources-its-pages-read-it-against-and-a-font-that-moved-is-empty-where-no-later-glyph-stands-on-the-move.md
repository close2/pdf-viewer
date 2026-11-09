# 2746 — A Type 3 font is given the resources its pages read it against, and a font that moved is empty where no later glyph stands on the move

Status: implemented. Session 1966, a single round, the R04 slot. The two items ADR 2500 left
refused and ADR 2523 did not reach. Context: ADR 0255 (a name nothing defines draws nothing), ADR
0816 (no content-stream byte is edited), ADR 0947 (a shape a remedy cannot carry out is a refusal
naming it), ADR 1059 (a Type 3 font stating no `/Resources` is read against the page's), ADR 2490
(R04's remedy), ADR 2500 (the empty resource and the empty font), ADR 2523 (a copy of a form per
page); `crates/pdf-archive/src/survey{.rs,/fonts.rs,/operators.rs,/records.rs}`,
`crates/pdf-transform/src/archive/reader_resources.rs`. ADR 2747 and Q808 are unused.

## A Type 3 glyph procedure's resources

ISO 32000-2 §9.6.4 (Table 110, `Resources`) reads a Type 3 font's glyph procedures against the
font's own resources, and a font stating none against the page's that shows it, which is where
§7.8.3's search ends; ISO 19005-2 and -4 section 6.2.2 ask the dictionary to be explicit and to
define each name. `assign` now writes it into the font dictionary, which is what ADR 2500 named as
the elegant remedy:

1. Where every page showing the font states one resources object, the font is given that object.
2. Otherwise the font is given one dictionary written for it: the union of the pages' category
   tables (§7.8.3's Table 34), each written directly in it. It reproduces every page only if no two
   pages bind a name to different objects, and no name a glyph procedure looked up and missed on a
   page is defined by another page while that page showed the glyph. The survey walks every
   procedure of a font it shows (6.2.2 binds them all), and the reader runs only those its codes
   select, so the survey now records, per page and per font, the procedures the shown codes select
   through `Encoding`'s `Differences` and `CharProcs` (`Survey::type3_glyphs_shown`); a code no
   `Differences` entry names, a direct procedure, or strings past the walk's budget count every
   procedure as shown.
3. A name no page defines is ADR 2500's empty resource, written into that dictionary.

Refused by name: a font written directly inside another dictionary, a procedure two fonts share,
and the union failing either condition of item 2 (`GLYPH_PAGES_DIFFER`), whose elegant remedy is
a copy of the font per page as ADR 2523 makes for a form; no census document needs it. Rewrites:
ADR 2500's `page-resources-assigned` and `empty-resource-defined`, losses none.

`pr4922` is the shape: one font, two pages, the procedure for `3` naming `/R98`, which only the
second page defines and only the second page shows. Its four outputs draw both pages pixel for
pixel as the source, and pass veraPDF; with the font's `/Resources` removed, veraPDF fails 6.2.2-2.

## A font that moved

ADR 2500 refused an empty font for a name nothing defines wherever a show moved the position by
more than the widths, and said nothing in the file could restate the reader's position. The reading
was too wide. §9.4.4's displacement is `tx = ((w0 - Tj/1000) × Tfs + Tc + Tw) × Th`: a `TJ`
adjustment and the line `'` or `"` starts do not depend on the font, and this reader applies both
with no font in force (`run_text_showing_op`), so the empty font moves alike. A character or word
spacing does move the empty font where the reader stood still, and that is a mark only where a
later glyph is placed from the text matrix it left. `BT`, `ET`, `Td`, `TD`, `Tm`, `T*`, `'` and
`"` set the matrix again from the line matrix or begin a new object (§9.4.2, Table 106; §9.4.3,
Table 107), and a show under an undefined name places nothing. So the survey keeps such a show
pending within its stream and records it only when a show under a defined font follows before one
of those; a form starts its own text object. A clipping mode is recorded at once, as before.

The govdocs files `020747`, `027613`, `189478`, `219789` and `492484` each set `Tc` under a
damaged font name and then `TD`. Their written outputs (`189478` at 2b and 4, `020747` at 4) carry
other remedies whose pixels differ: every page of `189478` and six of `020747`'s are photographed
(R03), and those pages' empty definitions are planned and not applied (`0 done` for `020747`).
The tests below are this construction's pixel proof.

## Proof

- Tests (`tests/archive/reader_resources.rs`): ten shows that move by `TJ`, a line start or a
  spacing followed by a reset are written with the empty font, conform at both parts, and draw
  the source's dark-pixel mask; five where a defined font's glyph stands on the spacing, or the
  clip, stay refused. A Type 3 font read on two pages is given the union, conforms and draws as
  the source; the three shapes of item 2 that fail stay refused; a name no page defines is an
  empty graphics state in the font's dictionary. Plants: the pending show never recorded fails the
  refusals and the pixel mask of `... 3 Tc (aaa) Tj /F1 24 Tf (a) Tj ET`; the union's check
  skipped fails the Type 3 refusals.
