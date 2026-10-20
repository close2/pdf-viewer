# 2106 — A clip that contains a mark does not band its draw

Session 1635, the second clause slot of batch ninety-nine, under the project owner's switch to code
improvements and bug fixes of 2026-10-14. Status: **accepted**, **built** in `render-cpu`. Clause:
ISO 32000-2 §10.7.4; the row stays `departed` (ADRs 1060, 1360) and its note gains the sentence.
Context: ADRs 0165, 0219, 1082, 1095, 1348, 2091. Code: `render_cpu::mask_cache::MaskCache::
effective`. Test: `render-cpu/tests/folding_stroke_under_a_clip.rs`.

## 1. What was found, and the hypothesis that did not hold

Round 1627 left one pixel: veraPDF's `PDF_A-4/6-3-1-t01-fail-f`, its paperclip annotation drawn by
the annotation path and by `Do` from the converted page, parted at 720 dpi at `(798, 567)` — black
`(0, 0, 0)` against `(5, 6, 15)`. The stroke alone, `S` for `B`, drawn from a page with and without
`0 0 7 17 re W n` reproduces it: 255 against 240 of ink, nothing else at 72, 288 or 720 dpi. The
pixel's area, the block that becomes it drawn sixteen and sixty-four times finer and averaged, is
**242.2** of 255 (242.23 to 242.32 at 16x, 242.16 to 242.24 at 64x, two converters). So neither
drawing is the integral: one is 12.8 levels over it, the other 2.2 under.

**The brief's hypothesis did not hold.** The clip does not change the converter. Both drawings take
the same one: the paperclip's top bends more tightly than its half-width, so it is stroked as ADR
1348's union of convex pieces, `scan::intersected` builds it in the coverage buffer, and `mask_fill`
hands the pieces to `tiny-skia`'s supersampled converter, which states a pixel in sixteenths
(`Exact::Pieces`, a cost decision of ADR 1348). What the clip changes is the **surface**. It removes
nothing, so `MaskCache::cuts_nothing` leaves it off the mark (ADR 1095) — but `effective` still drew
the mark into the clip's rows: a surface of 173 rows whose first row is 546 rows down the page,
reached by the page's matrix with 546 taken off its translation (`ty` 171.11963 for 717.1196). That
is ADR 0219's residual exactly — the same real number rounded in another binade — and an edge within
an `ulp` of a sample gave one drawing the sample and the other not: 16 levels, rounded to 15.

ADR 0165 is not the cause either. Its rule takes a `/BBox` clip off where the clip removes no
geometry, before the display list is built, so the annotation path's stroke carries no clip at all;
`Do` keeps §8.10.2's clip and `render-cpu` meets it. Both are right; they differed only in where
the second one drew.

## 2. Decision

**A clip whose region contains the mark is not composed into it, not built for it, and does not
band its draw**: `effective` answers such a command exactly as it answers an unclipped one, on the
surface's own rows. §10.7.4 says the intersection with a region containing the mark is the mark,
makes a pixel the half-open square on integer device coordinates, and says nothing that depends on
which row a surface begins at; a band is this program's division of the page, as a strip is, and
ADR 0219's rule for a division is that the answer is the one that exists without it. So the stroke
under its own `/BBox` is now the stroke drawn alone, to the byte, by every converter and not only by
the library's — `crate::area`'s one-level residual at a shifted origin goes with it.

The cost the band was kept for is the one an unclipped mark already pays. The draw is a view of the
surface's rows, which costs nothing per row: every converter bounds its work by the mark's own
reach. A command whose blend mode this backend computes draws into a layer of the band's height, and
that layer is now the surface's, as it is for the same command with no clip. The clip's mask is no
longer built for a mark it contains, which the band needed and nothing else does.

## 3. The two fixes measured and declined

- **Measuring the pieces by `crate::area`**, which is what §10.7.4 asks of every shape. It gives the
  witness pixel 243 under both drawings and, on `issue14415.pdf` at 4x, the worst pixel of that page
  0.682 against a ladder of 0.683 where the lattice says 0.88 — the lattice is up to 50 levels off
  there. But `callgrind_rasterise` (page 1, three draws): `inks.pdf` 281 584 529 → 30 980 638 917
  instructions, `rotated_ink.pdf` 16 955 208 → 72 697 996, `issue14415.pdf` 984 880 284 →
  1 879 490 287, `pr12564.pdf` 1 000 376 452 → 1 525 698 971; 59% of the first in `measure_
  overlapping_rows`. ADR 1348's reason stands, measured again; §4 is what it leaves.
- **Mapping the library's coordinates onto a device grid** (`f64`, rounded to 2^-9 or 2^-11 of a
  pixel), which would make every library-converted mark wholly inside two surfaces the same on both.
  The pieces overlap one another by less than the grid (ADR 1348's reach past a chord's end, an
  arc's sixteenth of 1/256), rounding turned overlaps into abutments, and
  `stroke_wider_than_its_curve`'s radius-20 disk drew 1255.23 and 1255.60 against 1256.72 ± 1.

## 4. What is left, stated

- A folding stroke's pixels are `tiny-skia`'s sixteenths, up to 50 levels from the integral on
  `issue14415.pdf`; a cheaper exact measure of a union of convex pieces is a later round's.
- A page drawn in strips, and a mark a clip *does* cut, still draw on a surface that starts down
  the page, and keep ADR 0219's residual; strips are the planner's and ADR 1758's bound holds them.

## 5. Measured

`folding_stroke_under_a_clip.rs` is the witness's own geometry and matrix, red on HEAD (`(798, 567)`
255 against 240) and green after. The witness converted under `as-shown`, page against annotation:
HEAD 0 pixels at 72 dpi and 1 at 720; after, 0 and 0; the annotation page itself unchanged.

**The corpus.** The six arms against HEAD's (`/home/AI/arms-1631/`): 0 of 968, 968, 968, 965, 965
and 965 frame digests move — they digest `render-raster`'s lanes — and the mean error against the
oracle moves on three pages, `issue14297.pdf` at 1x (compute 0.0219 → 0.0218), `issue19634.pdf` at
4x (cpu and gpu 0.0282 → 0.0284, compute 0.0112 → 0.0113) and `issue12798_page1_reduced.pdf` at 4x
(cpu 0.0021 → 0.0022). `raster_golden`, which digests the oracle itself: **42 of 974 first pages
move, raster only, every one by at most one level** — 1 to 269 pixels a page drawn in one strip
(`pdfjs_wikipedia.pdf` 269, `issue1905.pdf` 222, `issue3458.pdf` 95, `issue16263.pdf` 77,
`pr12564.pdf` 73; the rest under 55). Those three and `issue19634.pdf`'s glow at 4x were looked at
with every moved pixel marked: glyph and rule edges, the same page. That is `crate::area`'s own
residual at a shifted origin leaving every mark a clip contains, which is now drawn as the page
draws it.

**What it costs the division census**: the page drawn whole at 2, 4, 8 and 16 strips, HEAD 173
pages that move at some division, after 183, at most 943 pixels and 3 levels both, the same 11
named past ADR 1758's bound. A mark a clip contains was drawn at its clip's first row whatever the
division, so a strip drew it as the page did; it is now drawn on a strip's rows as every unclipped
mark is, and carries ADR 0219's residual as they do. ADR 0219's rule is the one that decides
between the two — the answer is the page drawn whole, and that is the answer this keeps.

**What it costs the page**, `callgrind_rasterise` page 1, three draws, HEAD → after: ISO 32000-2
page 101 4 816 901 061 → 4 763 971 411 (−1.1%), `issue840.pdf` 1 136 119 973 → 1 031 381 646
(−9.2%), `bug1844576.pdf` 8 800 654 → 8 447 859 (−4.0%), `bug1978317.pdf` 19 217 864 035 →
18 901 514 805 (−1.6%), `inks.pdf` +0.1%, `issue14415.pdf`, `bug1863910.pdf` and `issue21068.pdf`
within 0.06%. The saving is the clip masks no longer built for marks they contain.
