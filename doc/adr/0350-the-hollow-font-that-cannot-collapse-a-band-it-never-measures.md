# ADR 0350 — The hollow font that cannot collapse a band it never measures

Status: accepted, 2026-08-14. Session 515. Pins ADR 0216's decision against the shape it was
made for; amends no earlier ADR. §9.7.4.2's and §9.8.1's ledger rows carry the new witnesses.

## The question, which is another viewer's failure brought here

The project owner's brief (`tmp/ocr.txt`) describes a real production failure in a different
viewer. OmniPage CSDK was upgraded from version 20 to 22, and the scanned PDFs it produces
changed one thing: the invisible OCR text layer's `Identity-H` `CIDFontType2` went from **no
embedded program with `/CIDToGIDMap /Identity`** to an **embedded `/FontFile2` with a
`/CIDToGIDMap` remapping stream** — and the embedded subset is *hollow*: `head` and `hmtx`
present, every `glyf` entry empty. Text extraction is unchanged and correct everywhere. But
old PDFium derives each character's box from the font program, so `FPDFText_GetCharBox`
returns boxes with top equal to bottom: no selection overlay is painted (a zero-height
highlight), and `FPDFText_GetCharIndexAtPos` matches only on a razor-thin line. Acrobat and
newer PDFium substitute a face instead, which is why selection works there.

The question asked of this round: **what would this viewer do with such a file?**

## The answer, measured

Nothing collapses, at any layer, because this tree's selection geometry never asks the glyph
outlines for a vertical extent. Measured on a hand-built fixture pair in exactly the two CSDK
shapes (below), on this machine, this round:

- **Extraction**: `Interpretation::text` is `"Hollow scan"` — the fixture's exact text, both
  shapes, via §9.10.2's first method (`/ToUnicode`). Same as every other reader; nothing was
  ever at risk here and nothing is.
- **The text layer's geometry**: every `Placed` quadrilateral spans user-space y 147.516 to
  158.616 around the 150 baseline — which is exactly Table 120's band at 12 pt:
  `/Ascent 718`, `/Descent −207`, 11.1 pt tall. The hollow program contributes nothing to
  that number: `glyph_quad` takes its extent from `LoadedFont::extent()`, which is
  `vertical_extent`'s reading of the *font descriptor* through `measured_extent`'s
  plausibility band (ADR 0216) with the em box as the fallback. The outlines are consulted
  only to draw, and mode 3 draws nothing.
- **Selection through the real boundary**: in `viewer-core`'s headless harness, a pointer
  drag across the invisible word selects exactly that word, and every quadrilateral
  `Query::Selection` hands the host to paint is the descriptor's full 11.1 device pixels tall
  at scale 1. There is no razor-thin hit band to aim for either: `select::position_at`
  matches the *nearest* placed glyph with no distance threshold at all, so a click anywhere
  near the line lands on it — the failure mode the brief describes cannot occur by
  construction. (`Query::Caret`/`Offset` are §12.7's form-field questions in this vocabulary
  and do not apply to a page with no fields; text hit-testing is the pointer path measured
  here.)
- **The accounting** (ADR 0270's split): the invisible layer counts **nothing** —
  `codes_reaching_a_blank_glyph` 0, `codes_without_a_glyph` 0, no report, `glyphs` 0 — and
  that is correct rather than an omission: Table 104's mode 3 neither fills nor strokes, so
  no mark is owed and the blank-glyph band, which exists to count marks missed, has nothing
  to count. The same fixture with mode 0 is the discriminating control: all ten codes land in
  `codes_reaching_a_blank_glyph` (each reaches a glyph the program *contains* and describes
  with no contours — the program's own statement, §9.7.4.2's row), none in
  `codes_without_a_glyph`, and the font is reported as drawing nothing of what it was asked,
  which is trap 5 working.
- **The oracle's tolerance class** is also on the right side: since session 31 the text/vector
  question is `Interpretation::glyphs` — "did glyphs mark the page" — precisely so that "a
  page of invisible OCR text" is not classed as a text page (`doc/HANDOVER.md`, Things worth
  knowing). A CSDK-22 scan is judged as the image it shows.
- **The selection-geometry instrument** (ADR 0333) is not applicable to the fixture — its
  population is the pdf.js corpus — but it is the corpus-scale version of the same judgement,
  and it already judges this tree's word boxes against `pdftotext`'s over 507 documents.

So the finding is the good kind: **the defect the brief describes is a design this tree
already rejected**, in ADR 0216, for the owner's own report of other viewers' behaviour on
scanned pages — and what this round adds is the witness that keeps it true, because until now
no fixture existed in which metrics-derived and outline-derived bands *differ*.

## Why the answer is the specification's, not Acrobat's

The standard states no selection highlight and no character box — selection is a user
interface question (`select.rs`'s module comment says so and each choice is documented). But
the quantities the band is built from are the standard's, and they are derivable from the
file **without the glyph outlines**:

- §9.4.4's text rendering matrix places the band; §9.2.4 makes glyph space thousandths of the
  font size.
- Table 120 defines `/Ascent` as "[t]he maximum height above the baseline reached by glyphs in
  this font" and `/Descent` as the maximum depth — statements *about* the face, present in
  both CSDK shapes, and the CSDK-22 descriptor states a sane pair.
- §14.8.5.4.4 is the standard's own statement of what those two entries are worth as a line
  height, which is `measured_extent`'s anchor (ADR 0216).

Old PDFium's choice — measure the ink — is not *wrong* against any clause; it is a different
answer to a question the standard leaves open. But on this file the ink is a lie the
descriptor contradicts, and the descriptor is the entry the standard defines as the
measurement of the face. Acrobat's substitution repairs the drawing side; nothing here needs
repairing because nothing here asked the outlines. No code was changed by this round, and
none should be: substituting a face for the *invisible* layer, as new PDFium does, would be
work spent changing nothing a person can see.

## The fixture pair, and where it lives

`test-scenes::scanned_ocr_pdf(OcrFont, render_mode)` — the shared-fixtures crate, because two
test binaries need identical bytes. An 8×8 grey-checker image over the whole 300×200 page (the
stand-in scan), and under Table 104's mode 3 two words, "Hollow" and "scan", ten CIDs through
`Identity-H`:

- **`OcrFont::HollowEmbedded`** — CSDK 22: `/FontFile2` whose six tables are real (`head`,
  `hhea`, `hmtx` with distinct per-glyph advances, `maxp`, an all-zero short `loca` — every
  entry's start equal to the next, the table's own statement that every glyph is empty — and a
  zero-length `glyf`), plus a `/CIDToGIDMap` stream mapping CID c to glyph 11−c, the identity
  for no CID it covers.
- **`OcrFont::NotEmbedded`** — CSDK 20: no program, `/CIDToGIDMap /Identity`, same
  descriptor, same `/W`, same `/ToUnicode`.

Hand-built per trap 8, and "the corpus cannot supply this" is **counted rather than asserted**:
`examples/hollow_glyph_census` reads every corpus `CIDFontType2`'s own `loca` — a glyph whose
data starts where it ends is the table's statement that it is empty — and prints the two
populations and their intersection. Of 221 embedded programs it can read, 42 fonts in 30
documents reach their glyphs through a remapping stream, **214 programs have some empty glyph
and not one has only empty glyphs**, and the intersection is **empty**. That last number is the
whole justification for a hand-built file, and it was a claim about a population until this
round ran the sweep; the 214 is why the weaker shape is not a substitute — an ordinary subset
has empty glyphs, which is why `issue14821.pdf` exists in §9.7.4.2's row and why it does not
answer this question. The census reads the `loca` by hand rather than through `skrifa`, for the
reason `composite_fonts.rs` reads `hmtx` by hand: a sweep run through the renderer's own
library would measure the reader instead of the corpus.

Two constructions make the fixture discriminating rather than decorative:

- **The remap is measurable.** `/W` is keyed by CID, `hmtx` by glyph index, and the fixture's
  advances are distinct per glyph — so the document's width and the program's advance agree
  only through Table 115's stream, the same two-independent-statements check
  `composite_fonts.rs` runs over the corpus. A reader that ignored the stream would disagree
  with itself by up to 180 units per em.
- **The control mode exists.** The same file with mode 0 is what separates "nothing counted
  because nothing was owed" from "nothing counted because nobody looked": ten blank-glyph
  counts and a named report appear exactly when a mark is owed.

The tests: `pdf-model/tests/ocr_text_layer.rs` (extraction, the Table 120 band to the
hundredth of a point, the two-population accounting under both modes, the remap) and
`viewer-core/tests/headless.rs::a_drag_across_a_hollow_ocr_layer_selects_under_a_full_height_band`
(the whole journey: device pixels in, selected word and paintable full-height quads out). The
CSDK-20 shape's assertions are an equivalence — the substitute loads and everything holds, or
the font is reported by name — because §9.7.4.2 makes a substituted composite font reachable
only through what its codes mean, and which faces a machine offers is not this test's to
assume. On this machine the substitute loads and draws all ten codes.

## Consequences

- ADR 0216's decision now has the adversarial witness it was made for: a font program that
  states zero extents for every glyph while the descriptor states the truth. A future change
  that moved selection geometry onto the outlines would fail four tests by 11.1 points.
- `test-scenes` gains its second hand-built PDF and the `assemble_pdf` helper `basic_pdf` now
  shares; `viewer-core` gains a dev-dependency on it.
- §9.7.4.2's row carries the stream-plus-hollow witness beside its two corpus witnesses, and
  §9.8.1's — the row that describes the band itself — carries the file on which a
  descriptor-derived band and an outline-derived one differ for every character.
- Nothing in the drawing path changed; no corpus number can move (verified by the gates — the
  fixture is not in any gated population).
