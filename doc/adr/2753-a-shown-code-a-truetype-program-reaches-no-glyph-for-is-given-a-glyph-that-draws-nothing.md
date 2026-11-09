# 2753 — A shown code a TrueType program reaches no glyph for is given a glyph that draws nothing

Session 1971, a single archive round.
Status: **accepted and built** in `pdf-transform` (`archive/fonts/notdef_glyph.rs`: `NewOutline`,
`with_glyphs_given`, `outlines_asked`; `archive/fonts/truetype_cmap.rs`: `shown_codes`,
`codes_on_nothing`, `program_restated`, `restate_one`). It extends ADR 2543 under R03a of
`doc/pdfa-blocking-issues.md`.
Context: ADRs 2543, 2578, 2602; `doc/questions/A694` item 3.
Clauses: ISO 32000-2 §9.6.5.4; ISO 19005-2 sections 6.2.11.4.1, 6.2.11.5 and 6.2.11.8; ISO
19005-4 sections 6.2.10.4.1, 6.2.10.5 and 6.2.10.9.

## 1. What was refused, and why the premise was half true

The brief named R03a's remaining program kinds — a Type 1 `/FontFile`, an OpenType `/FontFile3`, a
CMap of the file's own. The census showed two of the three documents R03a alone refuses at both
parts, `issue14821.pdf` and `issue17333.pdf`, over a kind already built: a simple TrueType font
whose program ADR 2543 extends. What refuses each is a code that §9.6.5.4's route sends to no
glyph at all, not one on glyph 0 — `issue17333.pdf`'s code 0 under `MacRomanEncoding` (no name, and a
(1, 0) format 6 subtable that starts at code 165), `issue14821.pdf`'s `e`, `x` and `1` in a subset
whose `/Encoding` is a content stream. The requirement that names them is section 6.2.11.4.1's
"Embedded fonts shall define all glyphs referenced for rendering", which `pdf_archive` fails for a
code whose `glyph_index` is none. ADR 2543 gave a glyph only to a code on glyph 0 and, for a font
with none, kept the refusal, because a copy of `.notdef` would draw a mark this reader did not.

## 2. The construction

This reader draws no mark for such a code and advances by its width (`LoadedFont::outline` is none
for it: §9.6.5.4 names no glyph, and §9.6.5.2's substitution of `.notdef` is the Type 1 rule). So
the glyph the code is given has no outline: a `loca` entry spanning no bytes of `glyf`, which is
the TrueType Reference Manual's shape for a glyph without one, an advance equal to the code's
width in the program's units (one glyph per distinct width, section 6.2.11.5, as ADR 2543 section
2), and a left side bearing of 0. The restated `cmap` sends the code there, beside ADR 2543's
copies of glyph 0 for the codes that reached it. It is ADR 2578's "a glyph that draws nothing where
it reached no glyph", in a `glyf` program.

- **Only shown codes, only where the survey kept them whole** (`shown_complete`): a code never
  shown is no reference for rendering, and a prefix cannot prove the rest.
- **A shown code on no glyph that this reader drew a mark for** keeps the refusal: no empty glyph
  restates it. A font with any such code is left out of `notdef_given`, so the rows stay refused
  whole rather than answered for part of the font.
- **Only the file's own program**: a face this program embeds for an unembedded font is restated
  over its own table already (ADR 2744), and is not given empty glyphs.
- **The proof** is ADR 2543's, widened: each new glyph's outline in the restated program is glyph
  0's or empty as asked, the reloaded font reaches each given glyph by its own subtables, and the
  program's advance for each given code is its width within section 6.2.11.5's tolerance.

## 3. Each profile, and the loss

The answer is ADR 2543's `Rewrite::NotdefGlyphGiven`, stated, under every profile at both parts:
the page draws what this reader drew. It is not a lossless restatement for another reader, and
the stated sentence (`NOTDEF_GLYPH_REINTERPRETS`) now says so: a reader that drew the program's
`.notdef` for a code reaching no glyph — §9.6.5.4's last sentence leaves that choice to the
processor — draws nothing for it now. The file's `xmpMM:History` names the clauses and the fonts
as before; no date is invented.

## 4. Measured

Both documents convert at 2b and 4 under all nine profiles (`factur-x` declines every source of
this list for its own reason), and veraPDF 1.30.3 passes the four `as-if-printed` outputs. Drawn
on the CPU at 72 dpi beside the source: `issue17333.pdf` identical, `issue14821.pdf` five pixels
one level apart, the same with `/OutputIntents` renamed, the page's text otherwise identical —
including `text1`, which this reader and the output both draw as `t   t`; the five pixels lie in
rows the substituted Helvetica draws, not in the font this construction touches. Both censuses
against round 1970: 8 NEW each (the two documents on every line), 0 GONE, 0 CHANGED, and veraPDF
passes all sixteen. R03a now frees 2 at each part and stops 6 / 3 under `as-shown`.

## 5. What is left of R03a

`issue11768_reduced.pdf` shows a code that §9.7.6.3 reads as invalid under the file's own
`UniJIS-UTF8-H` CMap and sends to CID 0, in an `Adobe-Japan1` font with no `ToUnicode`. A fresh
CID there would need a codespace range the CMap does not state and a CID §9.10.2 reads as a
character, which ADR 2622 section 2 leaves refused; it is not built. Every other R03a document is
refused on another row as well.
