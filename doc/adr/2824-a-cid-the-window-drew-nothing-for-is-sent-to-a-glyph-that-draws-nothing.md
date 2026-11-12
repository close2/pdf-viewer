# 2824 — A CID the window drew nothing for is sent to a glyph that draws nothing

Session 2003, a single archive round.
Status: accepted and built. In `pdf-font`: `embed::for_embedding_with_empty_glyphs` (new) beside
`for_embedding`, both now through one writer that appends empty glyphs. In `pdf-transform`:
`archive/fonts/machine_cid.rs` (`shown` keeps a CID the face has no glyph for, `written` sends it to
an empty glyph), `archive/fonts.rs` (`SubstitutedFont::empty_cids`, `empty_cids_given`),
`archive/report.rs` and `archive/prepare/font_rewrites.rs` (reported and recorded).
Tests: `pdf-font` `embed::tests::empty_glyphs_follow_the_kept_ones_and_draw_nothing`;
`crates/pdf-transform/tests/archive/machine_cid.rs`
`a_cid_the_window_drew_nothing_for_reaches_a_glyph_that_draws_nothing`.
Context: ADR 2807 (the composite route this extends), ADR 2458 and `doc/questions/A48` (the empty
glyph of a simple font, and the condition it is given on), ADR 2773 (the proof), `doc/pdfa-blocking-issues.md` R09 (a).
Clauses: ISO 32000-2 §9.7.4.2, Table 115, §9.7.4.3, §9.9.2; ISO 19005-2 sections 6.2.11.4.1,
6.2.11.5, 6.2.11.8; ISO 19005-4 sections 6.2.10.4.1, 6.2.10.5, 6.2.10.9.

## Context

ADR 2807's route stopped wherever a page showed a CID the window drew no glyph for, and seven of
R09's documents did. A probe of each at HEAD read what the window drew for every such code: a CID
of the collection other than 0, no glyph of the machine's face (`DroidSansFallbackFull`, which
holds no Latin letter), no outline, and not §9.6.5.2's `.notdef` in place — so nothing, at the
advance `/W` or `/DW` gives the CID. The route could send the CID nowhere but glyph 0, the
`.notdef` §9.9.2 requires every subset to define, and section 6.2.11.8 forbids a text-showing
operator to reference it.

## Decision

1. **Such a CID is sent by the `/CIDToGIDMap` to a glyph of its own that draws nothing**: an empty
   `glyf` entry appended after every kept glyph (`loca` gives it no bytes, which ISO/IEC 14496-22
   makes a glyph without an outline), one for each advance such CIDs are drawn at, restated to that
   advance like every other glyph (§9.7.4.3; section 6.2.11.5). The page then shows what the window
   showed, and the proof of ADR 2807 holds it to the window code by code as before.
2. **It states no character**, which is A48's condition: a `CIDFontType2` reaches a glyph by index
   alone (§9.7.4.2), and the CID's meaning is still its collection's and the `/ToUnicode`'s, which
   the route does not touch. The CIDs are listed per font in the report and in the file's own
   `xmpMM:History`, naming the clause, as A48 asks of the simple font's empty glyph.
3. **A code the window drew a mark for without a glyph of the face's own stays off the route**, and
   so does a code its `CMap` sends to CID 0: that reference to `.notdef` is the content stream's.
4. **No new loss is named.** Against the window the output loses nothing: `as-shown` keeps what the
   screen showed, and the screen showed nothing there. What a reader with a face holding the
   character would have drawn is the loss `machine-face` already names, which this route exists
   for only where the profile withholds it (ADR 2774); `as-if-printed` keeps the pack's face.

## Consequences

Freed under every profile that withholds `machine-face`, at both parts (the round's record has the
census): `issue11555`, `issue13343` (whose "descriptor already written" was the pack's fallback
after this stop), `issue19182`, `issue6286`. `issue2128r`'s font now converts too, and the document
stays refused on section 6.1.8's names alone. `issue15053` and `mixedfonts` now stop
later: the empty CIDs are proportional Latin (0.604 and 0.278 em) and the face's em is 256 units,
which states neither within section 6.2.11.5's thousandth; the route falls back with the pack's
`machine-face`, and a face with a finer em would convert them.
