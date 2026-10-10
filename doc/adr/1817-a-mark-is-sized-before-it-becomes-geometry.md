# 1817 — A mark is sized before it becomes geometry: the one change the next batch builds

Asked by the project owner on 2026-10-10, as the step back of a performance batch: "analyze if
there is a big picture change we are missing, instead of doing adjustments to existing code" — by
first principles and by finding out how others do it — about loading and display time, "zooming in
and out especially", on `/tmp/demo_heavy_engineering_A1.pdf` and pages like it, on the processor and
on the device. Status: **accepted**, built nowhere yet: it picks the work, `doc/todo/69` names its
increments. Session 1490. Context: ADRs 0226, 0368, 0698, 0700, 1082, 1102, 1742, 1809, 1813, 1815.
Instrument: `crates/render-raster/examples/tile_price.rs`. Clauses: §8.4.3.3, §8.5.3.2, §10.7.4;
no ledger row moves.

## 1. What the window needs, against what this tree computes

The page is one 1684 × 2384 sheet drawn as 4 408 405 marks, 4 400 060 of them strokes: each one
`q 0.12 0 0 0.12 0 0 cm x y m x y+1 l S Q` under `2 w` and `1 J`, a round-capped segment 0.12 units
long and 0.24 wide, laid end to end in thousands of runs, so that a line on the drawing is a few
hundred dots. At the window's fit (1600 × 1000, 0.5938 px/unit, the page 1416 × 1000 px) each dot is
about 0.21 × 0.14 px, and the window has 1.4 M pixels for 4.4 M marks.

What is computed for it, measured on HEAD `bf2a9b00`'s export, pinned to the eight fast cores, the
minimum of three draws, in two clock holds at load 1.3–7.0 (`tile_price`, both runs' figures):

| stage | cost | what it is per mark |
|---|---|---|
| open and interpret, one thread | 2 267 / 2 553 ms | 29 M operators, a 4.4 M-command list |
| the device, whole fit frame | **refused after 2 232.8 ms** — 423 206 976 B against 268 435 456 | 96.0 B: six flattened edges of 16 B |
| the processor, whole fit page, one thread | 7 265.5 / 7 326.2 ms | **1 656 ns** |
| the same page with the 4 397 503 marks under a pixel left out | 44.6 ms | — |
| the floor: each of those marks blended into the pixels its extent meets | 138.2 ms | 31 ns |

**So 99.4 % of the processor's drawing of this page is spent on marks smaller than a pixel, at about
fifty times what blending their pixels costs**, and the device refuses the frame for the same marks:
its budget counts the six edges each dot's outline flattens to, and the flattening is host work paid
before the refusal. `quorra` then draws the whole arrangement on the processor, so its first frame
(12.00 s on Xvfb, 12.33 s on the owner's display, round 1489) is interpretation, refusal and the
7.3 s draw, and a zoom step pays the last two again.

**The census, by the longer side of each mark's ink** (its hull grown by half its width in the path's
own space, `tile_price` section 1): at the fit 4 365 065 marks (99.0 %) are under ¼ px and 2 927 922
lie wholly inside one pixel; at 150 dpi 4 365 176 are under 1 px; **at four times the fit 4 365 065
are still under 1 px**. The brief's hypothesis holds for its population — essentially every one of
the 26.45 M edges belongs to a mark under a pixel at window size, and at every zoom a person uses on
this sheet — and fails for its lever, as section 3 says.

**And what the 1 656 ns is made of**, by callgrind (`callgrind_rasterise --scale=0.5938`, one
draw, `RAYON_NUM_THREADS=1`, the same export): 157.9 G instructions, of which drawing is 112.4 G
(71.2 %) — 25.5 k a mark — and interpretation 43.5 G. Inclusive, as shares of the whole run:
`draw_stroked_outline` 42.0 % (ADR 1082's exact area, `scan::mask_fill`, 25.3 %; `fill_rect`
through a pipeline `RasterPipelineBlitter::new` builds per mark, 11.3 %), the stroker 13.4 % (its
round capper's conics 8.6 %), `convert::from_skia_path` 4.5 %. **Every row is the construction of a
general outline for a shape whose whole ink is one or two pixels**; none of it is the area itself.

## 2. How others do it — evidence about approaches, never about pixels (principle 5)

- **Every one of them presents from a picture, not from a render**: pdf.js scales the canvas it has
  by CSS and replaces it, and renders only the visible part at high zoom (`PDFPageDetailView`, still
  walking every operator); okular asks poppler for tiles and shows a partial rendering every 500 ms
  of a long one (KDE D8379, poppler 0.63's callback); pdfcraft draws pages over 4096 px as 1024 px
  tiles over a low-resolution backdrop from a pool of workers, stale rasters stretched until sharp
  ones land (`crates/ui-egui/src/canvas.rs`); mupdf runs one display list from several threads in
  bands (`mutool draw -B -T`) and culls a stroke against the scissor as it flattens it; pdfium and
  Foxit render progressively behind a pause callback (`FPDF_RenderPageBitmap_Start`).
- **None of them draws a sub-pixel stroke as a general one.** mupdf widens it before flattening —
  `if (linewidth * expansion < aa_level) linewidth = aa_level / expansion;` in
  `fz_draw_stroke_path_aux` — and that is a departure this tree may not take: it paints more area
  than the shape has. What is evidence is the *shape* of the answer: the thin mark is decided before
  the general machinery is entered.
- **`hayro`, the engine under pdfcraft**, opened, interpreted and drew this page at the fit in
  4 998.1 ms against this tree's 9 237.8 ms, one thread each, alternating in one process
  (`hayro-speed`, 1.85×). The witness's owner-reported gap is therefore not a gap in architecture
  alone: per mark, this tree is the slower of the two Rust readers.

## 3. The two candidates, priced on the witness

**A. The page as tiles from a spatially culled list, on a pool** (pdfcraft's and okular's shape).
`tile_price` sections 2–3: a window at four times the fit drawn from the whole list took 1 704.1 /
1 669.6 ms; from the 514 527 commands `device_bounds` lets reach it, 993.9 / 1 093.5 ms, the same
pixels (0 differ; the linear cull 64 / 70 ms, which a bin index makes proportional to the view);
cut 2 × 2 on four threads, 376.0 / 419.5 ms. At the fit nothing is culled — every mark is on screen
— and the 2 × 2 grid gives 3 102.5 / 3 382.1 ms, 2.2×, because the busiest tile holds 1 752 976 of
the 4.4 M marks. On the device each fit tile fits the budget and is drawn — 2 419.6, 1 232.9,
2 975.5, 1 425.0 ms, 8 053 ms in all, slower than the processor's one thread, because the device's
cost here is the same per-mark host encoding. **Worth the pool's width at best, and less at the
view a person opens the page in; it multiplies a per-mark cost rather than removing it.** Its
seams also move pixels: 588 at the fit and 778 at four times differ from the whole draw, the
strip-invariance question slot 2 of this batch is arguing for strokes.

**B. A mark sized before it becomes geometry.** Price: the 7 281.6 ms the sub-pixel marks cost
against a 138.2 ms floor. A closed-form kernel computing §10.7.4's exact area of the true shape
costs some multiple of the floor, not of the general path; at even three times the floor the fit
page's raster is 7.3 s → about 0.46 s, **≈ 16×**, at every magnification up to four times the fit,
on one thread, with no seam and no cache. On the device the same classification is one instance
per mark instead of six edges — at up to 60 bytes a mark the witness fits the stated 256 MiB
whole (4 408 405 × 60 = 264.5 MB) — and the host's flattening, 2.2 s of every refused frame
today, is not done.

**C, named and not priced here**: interpretation, 2.3–2.6 s, is what is left of the first frame once
B is built, and the next floor; drawing the list's prefix while the rest is read is okular's shape.

## 4. Decision

**The next batch builds B: a mark is classified at the device transform before any path, outline,
edge or brush is built, and a mark whose ink lies within a small block of pixels is drawn by a
closed-form exact-area kernel — on the processor straight into the pixmap, and in raster as a
primitive of its own whose budget is counted per mark.** A is the second change and multiplies B;
slot 5 of this batch starts its tiled entry, and it is sequenced after B rather than instead of it.

The kernel is held to the clause and not to the old bytes. §10.7.4:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects
> the shape, no matter how small the intersection is.

and the shape is the one §8.4.3.3's Table 53 states — "A semicircular arc with a diameter equal to
the line width shall be drawn around the endpoint and shall be filled in." — so the kernel's area is
the true capsule's, where the general path's is that of an outline the stroker flattened. Where
the two differ, the expected value comes from the clause (principle 5), and a digest that moves is
named against ADR 1742's bound rather than held.

**Level of detail stays forbidden.** The census's own population — 99 % of the marks under a quarter
pixel — is exactly what a level-of-detail scheme would drop, and ADR 0368 (e) read §10.7.4 against
it. This decision removes the *construction* of a dot, never the dot.

## 5. The first increment, its gate, its risk

- **First increment**: `render-cpu`, a single straight segment stroked with a butt, round or
  projecting cap, no dash, a solid paint, under a transform that keeps the axes, whose ink lies
  within a 2 × 2 block — the census's whole population on this sheet. Decided in `pdf-render` (trap
  2) so that the device lane's later primitive answers the same marks.
- **Gate**: a unit ladder of the kernel against the clause's closed form (sizes 0.05–2 px, positions
  across pixel edges, all three caps), the expected areas derived from §8.4.3.3 and §10.7.4 and not
  from the old path; `raster_golden` with its division test, every moved digest named; the six arms;
  `tile_price`'s fit row and `tools/state.sh heavy` as clock walks, before and after.
- **Risk**: pixels move on every page with small round-capped strokes, by the flattened cap's
  missing area — the price of being exact; a kernel admitted beyond its population draws a joined
  path wrong, which the ladder's control (a two-segment stroke must take the general path) guards.
