# 1754 — A stream-mapped CIDFontType2 is re-keyed, and a collection's named face is written out

Slot 3 of batch one hundred and fifteen, 2026-10-26, an archive round; ADRs 2342, 2343; no question.
I worked in `scratchpad/r1754/`. No ledger status moves; the §9.7.4.2, §9.7.4.3 and §9.9 notes grow.
**Re-keying (ADR 2342).** `archive::font_files::glyph_indices`: a `CIDFontType2` over a name-keyed CFF
whose `/CIDToGIDMap` is a stream is retyped `CIDFontType0` with its CIDs changed to the glyph indices
they selected — an embedded `CMap` per writing mode for each `Identity-H`/`-V` parent, `/W` by glyph
index — every code's glyph and advance read from `pdf_font::LoadedFont`; each refusal its own sentence.
**Collections (ADR 2343).** `pdf_font::embedding::collection_face` writes the face the descriptor's
`/FontName` names (ADR 0141) as one font, every checksum and `checkSumAdjustment` stated; face zero,
the reader's fallback, is not written. `doc/third-party-data.md` gains the two web-read texts' entry.
**Premises.** "Every CID draws the same glyph": held, for all 65 536 codes. "Sends CID *c* to glyph
*c* − 31": only the first of nine runs — its CIDs are Unicode values; the lever held regardless.
**Drawn** (looked at): page 1 of the three witnesses, source against both outputs, byte-identical PNGs
at 100 dpi and 360; veraPDF 1.30.3 PASS at 2b and 4 under both profiles (outputs byte-identical).
**Census** (converted / conforming / refused / unreadable), `doc/corpora` unchanged in all four:
| pdf.js | 2b `as-if-printed` | 4 `as-if-printed` | 2b `as-shown` | 4 `as-shown` |
|---|---|---|---|---|
| HEAD → this | 772/2/182/11 → 775/2/179/11 | 766/0/190/11 → 769/0/187/11 | 750/2/204/11 → 753/2/201/11 | 745/0/211/11 → 748/0/208/11 |
By document, every column: the three witnesses leave the refused list and none joins it.
**Proof.** `archive_corpus`'s `a_refiled_composite_font_draws_every_code_as_its_source_did` (ignored,
needs `doc/pdf.js`) holds every code to the source's glyph, advance, program advance and outline, and
a collection's program table by table to the reader's face. Plants (trap 13): a run off by one fails it
on `bug1186827`; face zero for the named face fails it on `issue13193`. `pdf_font::collection::extract`
keeps its behaviour, the face search factored out; the cpu-1x arm against HEAD's six arms: 968 pages,
0 digests differ (`pdf-font` is in `render-raster`'s graph, `pdf-transform` is not, `cargo tree -i`).
**Gates.** `rustfmt --check` on my files and `cargo fmt --check -p pdf-font -p pdf-transform` exit 0;
`-D warnings` clippy `--all-targets` on both exit 0; rustdoc `-D warnings --document-private-items` on
both exit 0; nextest on both 988 passed, 0 failed, 16 skipped. `tools/state.sh --round 1754 archive`
exit 0 in an export of HEAD and my files: validator over the veraPDF corpus missed 0, over 0 at all six
targets; converter 2b 526/85, 4 271/88, every conforming input still conforming; cross-check 13 560
compared, 0 disagreements. Census `--tree 6` exit 0 before (an export of HEAD) and after (the export);
two later edits, a doc comment and one refusal's own sentence, are not in it — unit and witness tests
rerun after them pass. `cargo test -p conformance --no-fail-fast` exit 101, one red, not mine: the
§10.7.4 row names ADR 2340, a sibling's, with no file yet (an earlier run's two reds were siblings' too).
`tools/state.sh docs` not run: rustdoc as above.
**Found, not fixed.** A re-keyed font under any other encoding, `/W2` or `/CIDSet` is refused, each
re-keyable the same way when a document asks. 5493 s.
