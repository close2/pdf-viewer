# 1264 — A `stream` keyword followed by a space, and seven notes whose figures are re-taken

Corpus slot, batch forty. ADR [1365](../adr/1365-spaces-before-a-stream-keywords-end-of-line-are-skipped-and-said.md).
No ledger status moved; §7.3.8.1's note gains the recovery (a recovery is not a requirement).

## Part A — §7.3.8.1

- `Parser::parse_stream_data` skips spaces and tabs between `stream` and a CR or LF, never past
  the marker; a `/Length` that fits only with the padding as data wins (direct in the parser,
  indirect in `Document::with_stated_length`). `Document::padded_stream_keywords` reports it.
- `tests/stream_keyword_padding.rs`: the pairs, and `4113564.pdf`'s six fax images decoding to
  every row (and failing with the padding prepended). 33 of 89 789 corpus files write the shape.
- Fuzz seeds are under `scratchpad/r1264/fuzz-seeds/`: `fuzz/corpus` is a link into the owner's
  checkout, which this round may not write.

## Part B — the seven notes

Re-taken on one oracle run, `render_at` ladders, `compare_rasters` and `magick compare -metric MAE`
over the gate's panels; every reference figure re-taken reproduced except `issue8697.pdf`'s
`poppler` (pdftoppm 26.08.0 now draws it 2.8 lighter).

- `CONTRADICTED_SUBPIXEL_IMAGE`: row 25 0.502 → 0.478 (ADR 1082); 8.49% → 8.06% against 7.33%;
  the mask still owns 1.3575; 32 → 16 pages.
- `AMBIGUOUS_GLYPH_COVERAGE`: ours now sits with `poppler` and `mupdf` at 72 dpi (40.93, 41.03), and
  the 5–6% "hinting" deficit is the page's partial last row (14.218 pt → 15 rows). ADR 1082.
- `AMBIGUOUS_NEAREST_THE_GEOMETRY`, `AMBIGUOUS_EVERYONE_OVER_THE_GEOMETRY`: ours re-taken;
  `issue12295.pdf` 7.681 → 7.000, on `mupdf`'s ladder (ADR 1102); step 7 −3.504.
- `AMBIGUOUS_STANDARD_FOURTEEN_FACE`: page 7 17.49 → 16.29; `issue16473.pdf` 1× 14.92 → 18.17.
- `AMBIGUOUS_SUBSTITUTED_FACE`: NuptialScript ours–mupdf 3972 → 1515; XiaoBiaoSong 10.75 → 10.03.
- `REFUSED_BY_THE_DEVICE_AT_FOUR`: a 4× run over its pages; two are refused for scene bytes.
- `CONTRADICTED_GLYPH_EDGES`: page 313's section says it left for `agrees` (ADR 1082).

`--bin overtaken`: 11 → 7; the seven left are other notes whose pages ADR 1347 or this batch's ADRs name.
`READ` gains seven entries (61 → 68).

## Left

`viewer-core` does not show the padding report. The ArialNarrow stem-run and pixel-count figures
had no recorded method; the counts were replaced by ink, the stem run was not re-taken.
