# 2302 — A collection's font is given a subset of the CJK pack's program, under `machine-face`

Session 1734, the archive slot of batch one hundred and twelve. Status: implemented in
`pdf-transform` (`archive::fonts::pack`, `tools/cjk-pack.py`). It answers the owner's A569. Context:
ADRs 0270, 2162, 2279; `doc/questions/Q569`, `A569`, `Q570` (open); `doc/history/1706`'s census.

## The reading

The largest refusal the real corpora met in pdf.js was a composite font the file does not embed
and nobody named a program for. Half of those fonts are `Adobe-Identity`, whose CIDs index the
producer's own program (§9.7.4.2); the rest name a predefined collection. For those, ISO 32000-2
§9.7.2:

> A character collection is an ordered set of glyphs. The order of the glyphs in the character
> collection shall determine the CID number for each glyph.

So a program whose glyph *N* is the collection's CID *N* carries the glyph each of the file's CIDs
names, whoever made it. §9.7.5.2 requires a processor to support `Adobe-CNS1-7`, `Adobe-GB1-5`,
`Adobe-Japan1-7` and `Adobe-KR-9`; every Korean `CMap` of Table 116 maps to `Adobe-Korea1`, so the
pack holds that collection as well — five programs.

## Decision

1. **The pack** is `doc/cjk-pack.md`'s directory: one OpenType program per collection with a
   CID-keyed `CFF ` table, its `ROS` the collection's and glyph *N* CID *N*, built by
   `tools/cjk-pack.py` from Source Han Sans 2.005R under the SIL OFL 1.1 — the family A569 names,
   read before it was chosen. A CID is placed only through a mapping Adobe publishes for the
   collection (its `Uni*-UTF32-H` and `-V` `CMap`s, `Adobe-Japan1`'s half-width ones, the IVD's
   `Adobe-Japan1` sequences); every other CID is an empty glyph. Filled, of the collection's CIDs:
   `Adobe-Japan1` 18 006 of 23 059, `Adobe-GB1` 28 669 of 30 283, `Adobe-CNS1` 18 574 of 19 178,
   `Adobe-Korea1` 17 009 of 18 351, `Adobe-KR` 21 010 of 22 896. Every charstring is written
   desubroutinized, its width its own first operand, because decision 2 restates advances and a
   width reached only through a subroutine is a site `pdf_font::restate` does not follow
   (`issue11555.pdf` was refused on it until then). One weight, Sans, is shipped: the
   standard names no face for a collection, so the choice is a documented one (principle 5), and
   it is the face the `machine-face` loss already says is not the producer's.
2. **The route** (`pack::embed`, reached from `composite::embed` where nobody named a program): a
   descendant whose `/CIDSystemInfo` is `Adobe` and one of the five, under a predefined `CMap` that
   is not `Identity-H` or `Identity-V` — §9.7.5.2 says those two "shall not be used with a
   non-embedded font", so such a file's codes are no collection's CIDs and it keeps its refusal.
   The shown codes become CIDs through the `CMap`; the subset (§9.9.2) keeps each at its CID
   through the program's charset and is written bare under Table 124's `CIDFontType0C` row; a
   `CIDFontType2` descendant becomes `CIDFontType0`, the subtype Table 124 puts a CFF program
   under, and its `/BaseFont` and the descriptor's `/FontName` become the subset's name (Table 117,
   §9.9.2). The Type 0 font's `/BaseFont` is the producer's and is kept. The program's advances
   are restated to §9.7.4.3's `/W` and `/DW` and proved, as on every other route. **One subset per
   descriptor**: the survey meets a font once per page, and two Type 0 fonts may share a
   descendant, so each meeting widens the one program to the union of the CIDs shown through it;
   two widths for one CID across them are ADR 0816's fence, refused by name.
3. **The loss is `machine-face`** (ADR 2162): `as-if-printed` discards it and `as-shown` stops. A
   shown CID the program has no glyph for is refused under both unless the collection's
   `Adobe-*-UCS2` table makes it a space, because that is a mark lost.
4. **The notice**: the OFL's condition 2 asks each copy to carry the copyright notice and the
   licence, so every subset's program stream carries `data/cjk-pack/LICENSE.txt` verbatim under
   `xmpRights:UsageTerms` (A550's pattern, `notices::attach_notice`); its condition 3 forbids a
   Modified Version the Reserved Font Name, so each program is `QuorraCJK-<Ordering>` (A550's rename
   precedent).

## A fault the route found in `pdf-font`

The first restatement of a CID-keyed program in this tree read back wrong: `cff::Layout::
with_charstrings` re-encodes the Top DICT, shifts everything after it, and copied the `FDArray`
verbatim — but each Font DICT names its Private DICT by absolute offset (Adobe Technical Note
#5176, section 19), so every Font DICT then pointed past its own `nominalWidthX` and local
subroutines. It now writes the `FDArray` afresh after the `CharStrings`, each `Private` offset
shifted, as `with_font_dicts` already did. `embed::cff`'s test
`a_cid_keyed_subsets_advances_restate_against_each_font_dicts_own_private_dict` holds it, and
fails against the old construction (planted). No earlier route restated a CID-keyed program —
the `--font` composite route refuses one by name — so nothing already converted is affected.

## Result

Witnesses, page 1 at 100 dpi, source against output, looked at: `noembed-sjis.pdf` (`Adobe-Japan1`,
`90ms-RKSJ-H`, `MS-Gothic`) and `issue8372.pdf` (`Adobe-GB1`, `UniGB-UTF16-H`) draw the same
characters at the same places in Source Han's shapes where this machine's Droid Sans drew them;
veraPDF 1.30.3 passes both outputs at 2b. `90ms_rksj_h_sample.pdf` stops on its inline descriptor,
refused by name. No `Adobe-CNS1` or Korean document is in either corpus. Of the 37 composite
refusals pdf.js met, 13 name a collection; with the pack 8 convert at 2b and 7 at part 4 under
`as-if-printed`, and under `as-shown` they stop at `machine-face` (the census is
`doc/history/1734`'s). veraPDF 1.30.3 fails three of the newly converted outputs at its 6.2.11.3.1
test, which compares the CIDFont's supplement to the CMap's the wrong way round: ISO 19005-2 section
6.2.11.3.1 asks the CIDFont's to be at least the CMap's, which all three meet, and this validator
reads it so (principle 5).
