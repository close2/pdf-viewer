# 2582 — The checker reads a shown CID 0 as the CIDFont's `.notdef`, by the CID the font draws

Status: implemented, in `pdf-archive` (`table::fonts::no_notdef_glyph_shown`) and `pdf-font`
(`LoadedFont::drawn_cid`). Session 1874, the third archive slot of batch 128.
Context: ISO 19005-2 section 6.2.11.8, ISO 19005-4 section 6.2.10.9; ISO 32000-2 §9.7.4.2 and
§9.7.6.3; ADR 2557, whose section 4 found the gap; ADR 2556, the copy it narrowed.

## The gap

`fonts/no-notdef-glyph-shown` judged a composite font by glyph index alone: a code was a `.notdef`
reference where its glyph was glyph 0. ISO 32000-2 §9.7.4.2 states that every CIDFont shall contain
a glyph description for CID 0, "which is analogous to the .notdef character name in simple fonts",
so in a CIDFont `.notdef` is a character, CID 0, and a code that selects it references `.notdef`
whatever glyph a `/CIDToGIDMap` sends CID 0 to. The checker passed exactly the output ADR 2557
withdrew: CID 0 sent to a copy of glyph 0. veraPDF failed that output on 6.2.11.8; the converter's
refusal already stood, and the checker shared the converter's earlier reading.

## Decision

1. **The CID read is the one the font draws.** §9.7.6.3 sends a CID the descendant has no glyph for
   to the `CMap`'s notdef mapping, and failing that to CID 0. `LoadedFont::drawn_cid` returns the CID
   whose glyph is drawn after those fallbacks, from the same function the drawing route uses
   (`composite_selection` in `crates/pdf-font/src/loading/glyphs.rs`), so the glyph drawn and the CID
   the checker reads cannot take two readings of the clause. `LoadedFont::cid` stays the `CMap`'s
   own answer, which the converter's map restatement needs.
2. **A code is reported where it ends on CID 0, or on glyph 0.** The two conditions overlap at CID 0
   on glyph 0, which is one finding; a CID above 0 on glyph 0 keeps the glyph reading (the TrueType
   program's `.notdef` by position), which ADR 2556's copy answers.
3. **A code that falls back to CID 0 is reported as one the `CMap` maps there.** The fallback ends at
   the same character, and §9.7.6.3's NOTE calls the notdef mappings the analogue of the `.notdef`
   mechanism.

## What it moves

No converter decision: `cid_notdef_glyph::given` refuses a font with a shown CID 0, and a font whose
only finding is a CID 0 on another glyph has no CID on glyph 0 for the copy to give, which is the
same refusal (`cids_on_notdef` answers `None`). A source that reached CID 0 on a glyph other than 0
and passed the checker before now fails it, and is refused where it was converted: that output was
one veraPDF fails. Such a font is refused with its own sentence, `cid_notdef_glyph::CID_0_SHOWN`:
the program holds the glyph CID 0 draws, so neither the fence's "a glyph the program does not
define" nor the copy's list of conditions describes it, and the route that would answer it is ADR
2557 section 3's embedded `CMap`.

## Measured

- **The census**, with and without the CJK pack, against HEAD's: every pdf.js line moves by one
  document, converted to refused, at 2b and at part 4 under both shipped profiles, and every
  corpora line is unchanged. The document is pdf.js's `issue7544.pdf`, whose `Identity-H` font
  shows CID 0 and sends it by its `/CIDToGIDMap` to the glyph of a `T`. veraPDF fails HEAD's output
  of it on 6.2.11.8-1 at 2b and 6.2.10.9-1 at part 4, so the move is a correction.
- **The plant on the real witness**: with `given` and its proof allowed a shown CID 0 again, round
  1861's `4387ba48` source converts to the output ADR 2557 withdrew, and the conversion's last
  check names `fonts/no-notdef-glyph-shown` as still failed and writes no file.

## Rejected

- **Reading `LoadedFont::cid` alone.** It is the `CMap`'s answer before §9.7.6.3's fallbacks, so a
  code past the end of a `/CIDToGIDMap` stream, which the font draws as CID 0, would pass.
