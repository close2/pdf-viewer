# 2458 — A code a substituted font's own encoding sends to .notdef is given an empty glyph

Status: implemented. Session 1812, the archive slot of batch one hundred and twenty-one. Context:
`doc/questions/A48` (the permission and its condition), ADR 0927 (the four permissions, this one the
thinnest), `doc/pdf-a-conversion-limits.md` section 2.2, ADR 2146 (the standard 14 by name alone),
ADR 2442 (the ranking that counted sixteen documents under the refusal).

## What the sixteen were

The census of real producers' files listed sixteen documents under "no face this program ships has
a glyph for every code it shows", the same sixteen at 2b and 4 under both profiles. Read code by
code, **no script is among them** — the brief's hypothesis did not hold. Fifteen show a code that
the file's own encoding sends to `.notdef` in any program, which no shipped face and no face named
with `--font` could cover:

- **a code the encoding names no glyph for** (twelve): a line feed under `WinAnsiEncoding` or the
  standard encoding in seven govdocs files (with 0x14 in one and 0x02 in another) and
  `embedded_zip.pdf`; 0x00 in `issue18059.pdf`; control codes in `ControlCharacters.pdf` and
  `standard_fonts.pdf`; codes 0200–0215 in `ZapfDingbats.pdf`, which
  ISO 32000-2 Annex D.6's table encodes for no glyph (the table lists 188 codes, the font's own
  metrics file 202, and the table is what this reader draws by);
- **a `/Differences` array writing `.notdef`** over a code the page shows (three): `issue4304.pdf`
  (code 32), `issue11403_reduced.pdf`, `443752.pdf`.

The sixteenth, `Deficient inflammasome activation…`, shows `Delta` in Times-Roman, which the shipped
serif face lacks and a machine face draws: the refusal and its sentence are right there.

## The decision

1. **Where the shipped face is a Compact Font Format one and the font states no descriptor, each
   such code is given an empty glyph** — `pdf_font::cff::with_empty_glyphs`, one Type 2 charstring of
   a width and `endchar` per distinct width the dictionary states for those codes, named `notdef`,
   `notdef.1`, … (outside the Adobe Glyph List, so §9.10.2's glyph-name method states no character
   for it), every glyph the face held kept byte for byte. The encoding is restated so the codes name
   those glyphs; the program is the font's own object, not the shared face. The page shows what this
   reader showed — nothing, at the stated width — and no `.notdef` is referenced (ISO 19005-2
   section 6.2.11.8, ISO 19005-4 section 6.2.10.9). The readback proof holds an outline-less glyph
   equal to no glyph and nothing else.
2. **A48's condition is met in both places it names**: the report lists the codes per font, and the
   file's `xmpMM:History` entry for the substitution names them and the clause.
3. **Where the face is a `glyf` one** (Helvetica's, under a non-symbolic `TrueType` dictionary),
   the refusal is the fence's: a name reaches a glyph there only through the character the Adobe
   Glyph List gives it (§9.6.5.4), section 6.2.11.6 admits no other `/Differences` name, so every way
   to an empty glyph states a character the file never stated — A48's forbidden half.
4. **At Levels U and A the empty glyph is not offered**: it states no character, and ISO 19005-2
   section 6.2.11.7.2 asks there that every glyph shown map to one, so the refusal is the target's
   (`NotThisTarget`), with 2b and part 4 named as the targets that take it — the reading
   `doc/pdf-a-conversion-limits.md` section 2.1 makes for an unnameable code.
5. **Where the font states its own descriptor**, the code is refused by a sentence of its own
   (`NOTDEF_BESIDE_A_DESCRIPTOR`, not built yet), which no longer tells the operator that `--font`
   resolves it; a face named with `--font` reaches `.notdef` at the same code.

## Consequences

- Freed, at 2b and 4 under both profiles: `ZapfDingbats.pdf` and `issue4304.pdf`. veraPDF 1.30.3:
  the eight outputs PASS. A planted name absent from the program also PASSes veraPDF, so on this
  rule veraPDF is no witness; this tree's own second verdict is, and it failed the planted-out
  construction at `fonts/no-notdef-glyph-shown`. Drawn at scale 2 against the source: `issue4304`
  identical, `ZapfDingbats` 12 and 10 pixels one level apart in the red header, the output intent's.
- The other thirteen keep a refusal: seven also stop at the content-stream fence
  (`graphics/only-operators-the-base-standard-defines`) and would not convert with the descriptor
  route built; six are the `glyf` half; one is `Delta`.
- `ZapfDingbats.pdf`'s fourteen ornaments at 0200–0215 are drawn by no glyph in the source as this
  reader draws it, because Annex D.6 encodes nothing there; the output keeps that absence. A reader
  drawing by the font's own metrics would show them in the source. The table is the standard's.
