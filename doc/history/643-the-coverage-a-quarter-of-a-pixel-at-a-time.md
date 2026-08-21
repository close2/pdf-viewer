# 643 — The coverage a quarter of a pixel at a time

One group of the oracle's contradicted pages, opened rather than believed:
`CONTRADICTED_ANTIALIASED_EDGES`, holding `colors.pdf` pages 1 and 2. Its name said the difference
was our anti-aliasing, softer than anybody's. It is a **quantum**, not a softness — `render-cpu`
rounds an edge's coverage to a quarter of a pixel — and the page's verdict belongs to neither,
because the exact closed form is contradicted there too. **Twelve for twelve on a group's name naming a
hypothesis rather than a diagnosis.**

Date: 2026-08-21. ADR: **0474**. No pixel moved.

## How the group was chosen

Trap 1's cheapest tell: a group whose note argues *another* group's mechanism. This one's last
paragraph read "the bound is what makes it fail at all, and it is trap 12's shape … a bound the two
least-anti-aliased renderers set for each other", which is `CONTRADICTED_TIGHT_CONSENSUS`'s sentence
in that group's own words, under a name asserting a cause about *us* that no line under it measured.
The same shape as session 583's, one group over.

## What the pages are, and the instrument they supplied

Sixteen axis-aligned rectangles apiece, no glyph, no curve, no image, no clip, under
`0.001968504 0 0 0.001968504 0 0 cm` — which is `1/508` — so every boundary lands at a *known*
fraction of a device pixel (198.4252 and 396.8504 across). A rectangle's coverage of a pixel is a
product of two overlaps, so the page composites into a **closed form** out of the file's own
arithmetic with no renderer in it. Two were written: the exact one, and the one `tiny-skia` measures
with four samples per axis at 0.125, 0.375, 0.625 and 0.875.

Against the oracle's own artefacts, by `raster_compare`:

| page 1 | vs exact | vs quarter-quantised |
|---|---|---|
| ours | mean 0.0406, max **33** | mean 0.0023, max **1**, ssim 1.00000 |
| `hayro` | mean 0.0015, max **2**, ssim 1.00000 | mean 0.0545, max 33 |
| `mupdf` | mean 0.0173, max 13 | mean 0.0526, max 25 |
| `ghostscript` | mean 0.1368, max 54 | mean 0.1791, max 64 |
| `poppler` | mean 0.2823, max 124 | mean 0.3313, max 130 |

Our raster **is** the quantised form to one level of 255 over half a million pixels; `hayro`'s is
the exact one. Page 2 says the same. So the five are not a spectrum with us at the soft end: from
the geometry at the worst pixel they rank `hayro` 2, `mupdf` 13, **ours 33**, `ghostscript` 54,
`poppler` 124 — two paint the shape's area, `poppler` snaps an edge to the nearest pixel boundary,
`ghostscript` supersamples and filters, and ours rounds the area to a quarter, up in most places and
**down to nothing** below an eighth of a pixel. Third of five, not the outlier the note described.

`render-quorra/examples/edge_coverage_ladder`, added here, says it without a document and names the
backend: at an edge moved a twentieth of a pixel at a time, `render-cpu` answers 0, 0.2510, 0.5020,
0.7529, 1.0000 on both axes and the graphics device tracks the fraction to a level of 255. The CPU
backend is this project's correctness oracle and is the less exact of the two at an axis-aligned
edge.

## Why nothing was fixed, and that is the finding rather than the omission

Because the exact form fails the gate's own bound too — page 1 ssim 0.98772 against 0.98862, page 2
0.98001 against 0.98402 — the consensus pair being `poppler` and `ghostscript`, the two furthest
from the geometry. A rasteriser painting precisely the covered area would be contradicted on both
pages. So the pages moved to `CONTRADICTED_TIGHT_CONSENSUS` with the calculation beside them, which
is what trap 12 asks for, and the quantum is recorded as a defect with a price rather than chased:
`doc/todo/11` item 7 states the cure (interior plus eight exact edge and corner pieces, the shape
`sub_pixel_bands` already uses) and the two costs that have to be measured before anybody takes it —
nine `scan::fill` calls where there was one on the commonest command in the corpus, and every
axis-aligned edge on every gated page moving.

## What was corrected, beyond the group

- §10.7.4's **ledger row**, whose departure (1) carried the spectrum-of-softness sentence as the
  clause's own corpus witness.
- `doc/todo/_scan-conversion.md`: departure (1) now states how finely "partly" is measured, and the
  "where the departure is visible" bullet says what the page actually shows.
- `doc/oracle-and-corpus.md`'s example of "a page contradicted by a departure this project decided
  on purpose" — which was this page, and was wrong twice.
- `pdf_render::collapsed`'s doc comment, which cited the group for "the coverage its area implies".
- The tally sentence in `doc/traps/pixels-and-rasterisers.md` and its twin in
  `doc/oracle-and-corpus.md`: eleven → **twelve**.

## Instruments added

- `crates/render-quorra/examples/edge_coverage_ladder.rs` — the coverage each backend paints at an
  edge placed every twentieth of a pixel, both axes, against the geometry.
- `crates/pdf-model/examples/compare_rasters.rs` — the oracle's own four measurements between two
  PNGs on disk, which is what lets a closed form be put to the gate's arithmetic instead of to the
  eye.

`tools/spec-errata emit` on the base standard carries no annotation against §10.7.4; the only
erratum in the §10.7 family is #371, on §10.7.2's flatness.
