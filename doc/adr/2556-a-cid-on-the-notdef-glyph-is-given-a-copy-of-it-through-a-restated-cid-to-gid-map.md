# 2556 — A CID on the `.notdef` glyph is given a copy of it through a restated `CIDToGIDMap`

Status: implemented. Session 1861; narrowed by ADR 2557 (a shown CID 0 keeps the refusal).
Context: ISO 19005-2 sections 6.2.11.4.1, 6.2.11.4.2, 6.2.11.5 and 6.2.11.8; ISO 19005-4 sections 6.2.10.4.1, 6.2.10.5
and 6.2.10.9; ISO 32000-2 §9.7.4.2, §9.7.4.3, §9.7.6.3, Table 115 and Table 124; A694 item 3; ADR 2543, whose
section 4 deferred this case.
Code: `crates/pdf-transform/src/archive/fonts/cid_notdef_glyph.rs` (new), `archive/fonts/truetype_cmap.rs`
(`restated`, `CmapRestated::descendants`, `notdef_fonts_have_truetype_programs`, `history`), `archive/prepare.rs`
(`SURVEYED`, the survey handed to the restatement), `archive/rewrite/values.rs` (the descendant's two entries),
`archive/decision.rs` and `archive/rewrite.rs` (the sentences), `crates/pdf-font/src/loading/glyphs.rs`
(`LoadedFont::cid`), `tests/archive/cid_notdef_glyph.rs`.

## 1. Decision

A `Type0` font over a `CIDFontType2` whose shown codes reach glyph 0 of its embedded `glyf` program takes ADR 2543's
remedy by CID instead of by code. `notdef_glyph::with_notdef_copied` is reused unchanged: one copy of glyph 0's outline
for each distinct width the shown CIDs on glyph 0 have (§9.7.4.3's `/W` and `/DW`, read through `LoadedFont::advance`,
rounded to the program's units), each copy's advance that width. The descendant then names a new descriptor (the new
`FontFile2`, and a `/CIDSet` with the given CIDs' bits set where the descriptor stated one, Table 124's layout) and a
new `/CIDToGIDMap` stream (Table 115's 2 x c layout). The content stream and `/W` are untouched.

- **Always a stream.** `/Identity` (named, or absent, which §9.7.4.2 reads as identity) cannot send a CID to a glyph
  past its own number, so it is written out for every glyph the program held, then the given CIDs are set.
  A stated stream keeps every entry and grows only to the highest CID given, bounded at 65 536 entries.
- **The population is the survey's shown strings**, as the checker's own rule reads them, and only where every entry of
  the font is `shown_complete`; a code left out could be one on glyph 0 that nothing gave a glyph.
- **Refused, by `CID_NOT_GIVEN`**: a descendant written inline in `/DescendantFonts` (the entries are written into the
  descendant object); two `Type0` fonts over one descendant; a code on glyph 0 that has no CID of its own (§9.7.6.3's
  fallbacks: no CID key restates it without moving every other code that falls back); one CID at two widths; a width
  past 16 bits of program units; more than 256 widths; a program `with_notdef_copied` refuses; a failed proof.
- **The proof** loads the `Type0` font again with every new object inline and holds every shown code to its old glyph,
  or to its copy with the copy's program advance within ISO 19005-2 section 6.2.11.5's thousandth of its width, and to
  its old width.
- The `xmpMM:History` entry names the clause and the descendant objects, as the simple case's does (A48's condition).

## 2. What a charstring writer would take (CFF and Type 1, not built)

The copy of glyph 0 is a table edit in a `glyf` program and a new charstring in the other two. A CFF (`FontFile3`
`/Type1C` or `/CIDFontType0C`) needs: the CharStrings INDEX rewritten with N copies of glyph 0's charstring (its width
operand restated against `defaultWidthX`/`nominalWidthX` of the right Private DICT, per FDSelect for a CID-keyed one),
the charset given N new SIDs or CIDs, FDSelect grown, and every offset in the Top DICT recomputed since the INDEX moves
the tables after it. A Type 1 program (`FontFile`) needs eexec decryption, charstring decryption (key 4330), a new
`/CharStrings` entry under a fresh name with the `hsbw` width restated, re-encryption and `/Length1`–`/Length3`
restated; a simple Type 1 font then needs its `/Differences` to name the copy, which is an encoding rewrite like
ADR 1942's. Neither is a change to the copy's argument, only to its writer.

## 3. Considered and not taken

- **Editing the content stream to stop showing the CID**: takes a mark off the page (ADR 2543 section 2).
- **Remapping every CID the map sends to 0**, shown or not: a subset font's map is mostly zeros, so this would copy a
  glyph for CIDs no page shows, and still miss a CID past the map's end.

## 4. Measured

- **The census**, no-pack and with the CJK pack, against HEAD's: each pdf.js line one document more converted (as-if-printed
  870 to 871 at 2b and 881 to 882 at 4 no-pack, 871 to 872 and 882 to 883 with the pack; as-shown 810 to 811 and 815 to
  816 both ways), the corpora lines unchanged. `rank.py`: R03a 24 to 23 documents under as-if-printed at both targets,
  alone 15 to 14, the same with the pack. The document is `issue10519_reduced.pdf`, CIDs on glyph 0 through a map stream.
- **The other five** `CIDFontType2` documents stay refused: `4387ba48….pdf` and `emi15514-sup-0002-SupinfoS2.pdf` show
  CID 0 (ADR 2557; the second also on `annotations/flags-entry-present`), `issue14821.pdf` on `NOTDEF_NOT_GIVEN`, a font
  of its three places being one the copy does not reach (which, not established), `issue11768_reduced.pdf` on `cid-system-info-agrees-with-the-cmap` refused by name (ADR 2520's early
  stop), `issue12963.pdf` on its CFF and Type 1 fonts (section 2).
- **veraPDF**: `issue10519_reduced.pdf`'s outputs compliant at PDF/A-2b and PDF/A-4; the planted input of `4387ba48`
  fails 6.2.11.8 among eight rules, and the copy's own output for it failed 6.2.11.8 alone, on CID 0 — which is how
  ADR 2557 was found.
- **`getrusage`**, debug builds beside HEAD's: 60 000 CIDs on glyph 0 at 256 widths 25 492 KiB against 25 132 KiB,
  at 60 000 widths 27 252 KiB against 25 872 KiB (both refused before the copy, the checker's place list being cut
  short); 32 CIDs at 32 widths, the most the copy is reached with, 18 816 KiB converted against 14 872 KiB refused.
