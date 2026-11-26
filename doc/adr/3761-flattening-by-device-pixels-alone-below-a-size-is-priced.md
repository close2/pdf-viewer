# 3761 — Flattening by device pixels alone below a size is priced, and the relative bound stays

Session 2265. Status: proposed — the owner's choice is `doc/questions/Q1086`; until it is answered the
tree flattens as raster's ADR 0044 and ADR 1443 say. Context: A1073 (option 3, "price, not land");
ISO 32000-2 §10.7.2, §10.7.4; raster's ADR 0044, ADR 1443, ADR 3705; the turn gate's
`ContentStreamNoCycleType3insideType3.pdf` p1.

## Every flattener, and which has the bound

| backend | where | bound on a curve, in device pixels |
|---|---|---|
| raster, processor lane | `raster/crates/raster-gpu/src/raster/flatten.rs` (`cubic_tolerance`) | `min(0.25, extent / 32)`, `extent` the control polygon's diagonal — the relative term binds below 8 px |
| raster, compute lane | `raster/crates/raster-gpu/src/compute.rs` (WGSL `cubic_tolerance`) | the same, statement for statement |
| raster, device coverage lane | `raster/crates/raster-gpu/src/outline.rs` | no chords: cubics become quadratics drawn by an implicit test (raster's ADR 0016) |
| CPU oracle | `crates/render-cpu/src/area.rs` (`FLATNESS`), `stroke/folding.rs` (`FOLD_FLATNESS`, `ARC_FLATNESS`, the turn bound) | absolute: 1/256 for fills and arcs, 1/20 for a folding stroke's chords — no relative term |
| `render-gpu` | vello 0.9's `flatten.wgsl` | absolute 0.25 — no relative term |

So the relative bound is raster's alone, and the prototype touched raster's two flatteners and
nothing else. `render-cpu`, `pdf-model` (whose `raster_golden` draws with the oracle), `viewer-host`
(the strips test) and `render-gpu` do not depend on `raster-gpu` at all (`cargo tree`), so option 3
cannot move a pixel of theirs.

## What the relative bound is for, today

ADR 0044 added it because a quarter pixel drew a one-pixel circle as its inscribed square, 36 % short
of its ink — what §10.7.2's NOTE 2 calls drawing "inscribed polygons". ADR 1443 then made each flat
piece two chords through the midpoint of its inner controls, which encloses the piece's area to second
order, and kept ADR 0044 for the polygon's *shape*. Modelled on the four-cubic circle with both
constructions as they stand (`circle.py` beside the record): with the relative bound a circle under
five pixels across is 16 pieces, area +0.02 %, farthest point 0.002–0.014 px from the circle; by device
pixels alone a circle of diameter 0.5 or 1 is 4 pieces, area −1.18 %, farthest point 0.024–0.049 px, and
one of diameter 2 is 8 pieces, −0.05 %, 0.026 px. Both stand well inside the quarter pixel §10.7.2's
distance is held to on every larger curve. What the relative bound buys now is that last factor of
four in shape on curves under 8 px, which is where text at body size lives.

## The prototype

`QUORRA_FLATTEN_DEVICE_ONLY_BELOW=<px>`, read once into a `OnceLock<f32>` (zero when unset), and
nothing else: in `flatten.rs`'s `cubic_tolerance`

```rust
let extent = width.hypot(height);
if extent < device_only_below() {
    return FLATTEN_TOLERANCE;
}
FLATTEN_TOLERANCE.min(RELATIVE_FLATTEN_TOLERANCE * extent)
```

and the same three lines in the WGSL `cubic_tolerance`, whose module gains
`const DEVICE_ONLY_BELOW: f32 = …;` beside `EMIT` where `compute.rs` builds the flatten pipelines.
Unset, nothing changes. It was not kept in the tree: a switch nobody ships is a configuration surface
with no decision behind it, and these lines rebuild it. The corpus sweep was an example that drew
every tracked document's first page through `render_raster::options()`, wrote or compared RGBA, and
dropped its device every forty pages to let the lock go.
Three thresholds were priced: 1 px (a curve inside one pixel's diagonal), 2 px (the glyph of the
Type 3 page), 8 px (the relative term removed outright, since it is inert at and above 8).

## The price

The Type 3 page, the turn child of `tests/turn_path.rs` run directly, three interleaved passes under
one clock hold (`tools/bounded.sh --lock --clock`), milliseconds:

| below | turn | seventh | step |
|---|---|---|---|
| off | 2386–2628 | 2370–2684 | 855–933 |
| 2 px | 896–1119 | 937–1265 | 473–578 |
| 8 px | 941–1524 | 938–1288 | 465–533 |

Instructions for one draw of that page through `render_raster::options()`, callgrind, one rayon
thread, whole process: 62.32 G off, 35.01 G at 2 px, 34.96 G at 8 px; `meet::exact_areas` inclusive
35.93 G, 22.41 G, 22.35 G. The exact meet ADR 3705 measures is still 64 % of the draw with the switch
on: option 3 halves the pieces, not the measuring.

Pixels, every tracked corpus document's first page at scale 1 through raster's shipped options, the
switch against a baseline, and a second baseline as the control (0 of 957 pages moved):

| below | pages moved of 957 | worst channel ≤ 2 | 3–7 | ≥ 8 | pixels moved, median / largest page |
|---|---|---|---|---|---|
| 1 px | 522 | 437 | 80 | 5 | 26 / 8 391 |
| 2 px | 593 | 173 | 409 | 11 | 103 / 21 673 |
| 8 px | 616 | 59 | 410 | 147 | 207 / 30 708 |

The pages that move are text: `tracemonkey` and its variants move some 21 600 pixels by 1–6 levels.
The largest single moves are the Type 3 page (42 levels on its tiling's rim), `issue14415` (41, small
annotated text), `pr12564` (35) and, at 8 px, `rotated_ink` (90). `issue18032` moves two pixels by
220: a stray dark pixel inside a white glyph that the baseline draws too, one position over — reported
for a round of its own rather than priced here.

Against the CPU oracle, which flattens to 1/256 px and is the nearer of the two to the curve: of the
593 pages the switch moves at 2 px, 379 move away from the oracle by mean error, 21 toward it and 193
by less than the fourth decimal (worst tile: 189 away, 33 toward); at 8 px, 494 away and 14 toward. The
Type 3 page itself goes from 0.0178 to 0.0228 mean, 1.55 to 1.90 worst tile.

The turn walk whole (`tests/turn_path.rs`, the merge's command) with the switch at 2 px: 33 figures
banded, 30 judged, 3 outside — the Type 3 page's turn 885, seventh 907 and step 457 ms, each below
the band A1073's option 1 gave it; every other page inside its band, EC3 p101 not judged (load).

The launch walk whole (`PDFVIEWER_LAUNCH_CLOCKS=1`) with the switch at 2 px: 64 figures banded, 20
not judged for load, 4 outside. Two are the memory rows of `PDF20_AN001-BPC.pdf` and
`Well-Tagged-PDF-WTPDF-1.0.pdf` below their bands, as on two neighbouring rounds' walks without the
switch the same afternoon; two are those documents' page turns past their high edges (15.3 ms against
8.2–12.9 at load 31, 18.3 against 9.9–16.3). Fewer chords cannot make a turn slower by any mechanism
this round can name, but a turn miss is a regression until a comparison clears it. The control — the
same walk without the switch, on the same tree, an hour later — turned the same three documents in
11.5, 11.8 and 12.2 ms, inside, with one figure of 64 not judged for load where the switch's walk had
twenty. The two walks differ in the switch and in the machine's load at once, so they do not clear
the misses and do not convict the switch either; a round that adopts option 3 owes the comparison at
equal load, page by page.

Gates the switch reaches: raster's `a_circle_deposits_its_own_area_at_every_size` (a dot of diameter
0.5 draws 0.1922 against 0.1964, outside its byte rounding) and
`a_large_curve_keeps_the_segment_count_it_had` (9 points where 33) fail at 2 px; `raster_golden` and
the strips test cannot move (above).

## The recommendation

Not to adopt it. Option 3 buys a factor of about 2.5 on one page that stays thirty times past the band
it had before ADR 3653, and the exact meet keeps two thirds of the work; for that it moves the pixels
of six in ten corpus pages, almost all of them text, and on nearly every one of them away from the
oracle — the shape ADR 1443 kept the relative bound for, given back. If the owner wants the page faster, the measuring is where the time is.
