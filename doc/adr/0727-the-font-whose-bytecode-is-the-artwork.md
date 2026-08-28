# ADR 0727 — The font whose bytecode is the artwork

Status: accepted, 2026-08-28. Session 792. Amends the ledger's §9.6.3 row; settles all four
rows `doc/todo/03` §27 left "placed but not settled".

## What changed, in one line

**A TrueType font whose glyph shapes are computed by its instruction programs is now drawn
through the interpreter.** `LoadedFont::build_outline` drew every sfnt outline unhinted; for
the hint-reliant family — DynaLab's DFKai-SB and its relatives — the uninstructed `glyf` data
is a kit of stroke skeletons, not a picture, and drawing it put thin, misassembled characters
on the page in silence.

## The witness, and how it was found

`doc/todo/03` §27 left four crawl-head rows placed but not settled, and this round took them.
`corpus-cache/safedocs/cc-main-2021-31/7803/7803013.pdf` — a school class list, "Microsoft:
Print To PDF" — ranked −2.606 against the lightest reference, and its ladder against `pdftoppm`
(−9.19, −2.95, −2.53 at 1×, 2×, 4×) shrank without converging: a persistent −2.25 at 8× that no
scan-conversion mechanism explains. At 8× the difference is visible as *shape*: our CJK glyphs
were thin, their components small and scattered — `晞` drawn as a detached `日` beside a `希` —
while `poppler` and `mutool` drew the same well-formed Kai-style characters.

The font is embedded and subset: `DFKaiShu-SB-Estd-BF`, family `DFKai-SB`. Its glyphs are
composites of ten-odd stroke-component glyphs apiece at plain x,y offsets, and drawing exactly
what the `glyf` table states — verified independently with fontTools' decomposition —
reproduces **our** render, not the references'. FreeType renders it correctly *even under
`FT_LOAD_NO_HINTING`*, which is the tell: the face carries `FT_FACE_FLAG_TRICKY`, FreeType's
marker for fonts whose "font programs" must run to produce correct contours, and for those
faces FreeType refuses to skip the bytecode. The instructions are not grid-fitting an existing
outline; they *assemble* it — the artwork is in the machine code.

## The decision

`skrifa` — this project's own outline library — carries the same detection:
`OutlineGlyphCollection::require_interpreter()`, whose documentation states the contract:
when it answers true, hint through `Engine::Interpreter` with `Target::Mono`. The change
follows it, and only it:

- **Condition**: `require_interpreter()`, computed once per loaded font (it walks the name
  table and checksums `cvt `/`fpgm`/`prep`) and cached in a `OnceLock` beside the outline
  cache. Every font outside the family draws exactly as before, byte for byte.
- **Size: one pixel per design unit** (`Size::new(units_per_em)`), and this is the decision
  the round had to make rather than copy. Hinting is defined at a device size, and this
  crate's outlines are resolution-independent — built once, cached per glyph, scaled by the
  text matrix. At ppem = upem the pixel grid *is* the design grid, so the grid-fitting half of
  hinting rounds to the coordinates the font already states — a near no-op — while the
  constructive moves this family's programs exist for are carried out in full. The outline
  stays resolution-independent, the per-glyph cache stays valid, and the grid-fitting this
  tree has always declined (`doc/todo/_scan-conversion.md`) stays declined.
- **Failure falls back to the skeleton.** A family font whose `fpgm`/`prep` fail to run, or a
  glyph whose program errors, draws the unhinted outline — the picture every glyph of the
  family drew before this ADR — rather than nothing.

## What it is not

- **Not hinting.** No glyph is grid-fitted to the device; the choice of ppem = upem exists
  precisely to keep the interpreter's shape construction and discard its quantisation. The
  references make the other choice — FreeType runs the family's bytecode at the device size —
  which is why `pdftoppm`'s 1× render of the witness is 7 ink points *heavier* than its own
  8× render, while ours is scale-stable to 0.04.
- **Not a reading of another renderer as truth.** FreeType's behaviour was evidence that the
  bytecode is load-bearing; the diagnosis rests on the font's own data (the uninstructed
  outlines are stroke fragments) and the fix on our own library's stated contract.

## Measured

On the witness, page one at 72 dpi, ink as `255 − mean luma`, this round's renders:

| scale | before | after | `pdftoppm` | `mutool` |
|---|---|---|---|---|
| 1× | 16.21 | 18.47 | 25.43 | 18.85 |
| 8× | 16.28 | 18.51 | 18.52 | 18.52 |

At 8× — where grid-fitting has faded out of the references — all three renderers now agree
within 0.02 ink points. The row seeded in `doc/checks/fixed-documents.toml` pins 18.47 ± 1.

Two hermetic tests carry the mechanism onto machines without the crawl
(`loading.rs::a_hint_reliant_familys_instructions_construct_the_outline`,
`::a_family_font_with_a_broken_program_falls_back_to_the_stated_outline`): a two-glyph fixture
font whose one instruction program moves the square's top corner a tenth of an em — under the
family name the interpreter runs and the corner moves; under an ordinary name the stated square
draws; and a family font whose `fpgm` cannot run still draws the skeleton. Both calibrated by
planting each defect shape and watching exactly the right test fail.

Cost, `callgrind_interpret` (fifty interpretations of ISO 32000-2 page 101, no family font,
`RAYON_NUM_THREADS=1`, A/B one sitting): 1,187,649,176 → 1,191,309,728 instructions,
**+0.31%**, of which 0.21% is the once-per-font-load detection under the `OnceLock` and the
rest is codegen movement (`show_text` itself shrank 3.1M). In the product a font loads once
per document (`pdf_model::FontCache`, ADR 0710). On the witness itself, interpretation goes
6.2 ms → 14–16 ms — the family's own bytecode, paid only by documents that embed it.

## The other three rows, settled by construction

The same session settled §27's remaining three, each with the instrument the file named:

- **`7557015.pdf`** (−3.26): not resampling. Its photos agree with `pdftoppm` to 0.33 ink
  points; the whole gap sits in a band of thousands of 0.227 pt white *stroked* diamond
  outlines — sub-pixel strokes crossing and abutting, §10.7.4's anti-aliasing departure plus
  ADR 0308's composited boundaries, halving per doubling exactly as recorded. A decision
  already argued (`doc/todo/11` item 5, `doc/todo/_scan-conversion.md`).
- **`7557305.pdf`** (−4.00): not the colour conversion. A probe built from the page's own
  `/Separation` (`PANTONE 2945 C`, Lab alternate, Type 2 tint transform) renders **identically
  to `poppler` and `ghostscript` here — (0, 75, 152), our own closed form** — and 13 levels
  from `mutool`. The page differs because its *page-level transparency group* states
  `/CS /DeviceCMYK`: adding that one entry to the probe reproduces our page colour
  (15, 83, 143) exactly, while `poppler` and `mutool` return their direct answers unmoved
  (they do not composite the page in its stated group space) and `ghostscript` returns a
  fourth answer, (2, 81, 138), through its SWOP tables. §11.4.7's conversion through the
  blending space, by this tree's argued CMYK arithmetic against references that skip it or
  route it through ICC — trap 9's family, with the mechanism now reproduced by construction.
- **`7557122.pdf`** (−6.28): the darkest few percent of the ICC route, ADR 0510's standing
  finding, on the document's own data. The poster's ground is vector rich black through an
  `ICCBased` FOGRA-family CMYK profile under `/RelativeColorimetric`; a probe embedding that
  very profile puts all three references — one Little CMS between them, trap 9 — at (9, 0, 0)
  where we sit at (19, 14, 13), ~10 levels apart on the near-black patches and ~3 on the
  lighter one. On plain `DeviceCMYK` patches of the same values we and `poppler` agree byte
  for byte. An argued position (ADRs 0456, 0484, 0510), not a misread clause.

None of the three changes code; `doc/todo/03` §27 now carries each cause where it carried
"placed but not settled".
