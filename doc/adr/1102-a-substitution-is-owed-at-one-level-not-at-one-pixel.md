# ADR 1102 — A substitution is owed at one level, not at one pixel

Status: accepted, 2026-09-15. Session 1088. Moves the width at which ISO 32000-2 §10.7.4's
restatement of a too-thin mark is owed on `render-cpu`, the backend `CLAUDE.md` principle 2 makes
the correctness oracle, from one device pixel to one *level* of one — so that a sub-pixel stroke's
ink becomes the same function of the stated geometry on both backends. Takes six pages off
`render-raster --test corpus`'s differing list, 14 → 8.

## What a substitution is for, and what changed underneath it

§10.7.4 gives its own purpose:

> This ensures that no shape ever disappears as a result of unfavourable placement relative to the
> device pixel grid, as might happen with other possible scan conversion rules.

Under the anti-aliasing departure this tree takes (ADR 0025, §10.7.1's NOTE), a mark disappears when
its coverage rounds to nothing — so a substitution is owed exactly where the raster cannot state the
coverage the shape's area implies, and nowhere else. **That boundary is a property of the device,
and this backend's device moved.** ADR 0226 and ADR 0268 were written against `tiny-skia`'s
supersampled path converter, which states a general edge's coverage only on a lattice of sixteenths
and draws every stroke at or under one device pixel as a *hairline* carrying `cos θ` of a turned
rule's area. ADR 1082 gave this backend `render_cpu::area`, which states the winding integral over a
pixel exactly, down to the eight bits the raster itself carries. The sixteenth is gone; what is left
is one level of 255.

So `pdf_render::unmeasurable_width` is `substitute_width` × `1/255` — one level across whichever way
the mark runs — and between it and one whole device pixel a stroke is drawn as **the shape the
document states**: `sub_pixel_bands`' closed form where the mark is an axis-aligned rectangle, and
the stroke's own outline filled through `render_cpu::area` otherwise. That second construction is
what `render_raster` has always drawn from the same stated width (`stroke::resolve_width`, ADR
0701), so the two backends now answer one question with one answer rather than with two.

## Why the widened band could not stay, and it is not the ink of one mark

ADR 0268's identity is exact and this ADR does not dispute it: widening a mark by a factor and
dividing the paint's alpha by the same factor leaves its total ink where it was, at every angle and
under every transform. **What that identity is silent about is where the ink lands**, and §10.7.4's
first sentence is not:

> A shape shall be scan-converted by painting any pixel whose half-open square region intersects the
> shape, no matter how small the intersection is.

The sentence names the pixels a shape affects; a pixel the shape does not intersect is not among
them. A 200-unit rule at 5°, 0.2 of a device pixel wide, meets 248 pixels; the band that rule was
widened to marked 400, of which **144 lie further from the rule than half its width**.
`render-raster/tests/sub_pixel_coverage.rs`'s
`a_sub_pixel_rule_marks_only_the_pixels_its_own_band_meets` is that measurement as a gate, with the
old boundary as its control.

**And ink that lands in the wrong pixels is not conserved once the page has anything else on it.**
§11.3.6 composites two objects painted over one another as `1 − (1 − a)(1 − b)` per pixel, which is
not linear in the coverages — so moving a mark's ink onto pixels the shape does not cover changes
every later composite. `standard_fonts.pdf` is the witness and the arithmetic is visible in one
measurement: its marks drawn one at a time sum to **48 418.38** of ink before this change and
**48 530.71** after, a difference of 0.2%, while the *page* composites to **31 937.84** before and
**29 674.33** after — 7.1%. The page draws every table rule twice, 0.57 of a device pixel wide, once
for each of the two cells that share it; spread over a whole pixel those two draws lose less to each
other than they do where the document put them.

## What it moved, measured

`render-raster/examples/ink_ladder`, page one, scale-normalised, cpu oracle against raster:

| page | cpu 1× before | cpu 1× after | raster 1× | cpu 8× |
|---|---|---|---|---|
| `standard_fonts.pdf` | 31 937.84 | **29 674.33** | 29 627.02 | 21 206.92 |
| `issue12295.pdf` | 13 450.96 | **12 866.31** | 12 834.50 | 12 686.78 |
| `issue4402_reduced.pdf` | 1214.50 | **1201.65** | 1192.61 | 1215.50 |
| `issue11473.pdf` | 1486.50 | **1512.24** | 1493.40 | 1474.78 |
| `issue16038.pdf` | 231.12 | **230.24** | 229.54 | 238.02 |
| `issue18030.pdf` | 397.76 | **398.79** | 395.76 | 402.88 |
| `issue20232.pdf` | 18 050.69 | **19 324.45** | 23 703.71 | 18 242.24 |

The first two were 7.8% and 4.8% apart and are now 0.16% and 0.25%; the six above `issue20232` all
left `render-raster --test corpus`'s differing list, which reads **943 agree, 8 differ** where it
read 937 / 14. `issue20232.pdf` did not leave — raster is 30% long on it for the reason
`doc/QUORRA_FEEDBACK.md` §45 states — and it is the one page whose cpu total moved *away* from its
own 8× figure, from −1.1% to +5.9%, because its frame rules are strokes whose expanded outlines
overlap at every join and `render_cpu::area` declines such a path to `tiny-skia`'s quarter lattice
(ADR 1082's own decline, §11.6.2). That is the permitted side of "[t]he area covered by painted
pixels shall always be at least as large as the area of the original shape", and it is the same
defect on both backends rather than a new one on this.

**`22060_A1_01_Plans.pdf` is not in the table and that is the round's other finding.** Three
sessions named it as this decision's witness — "the two backends take different §10.7.4
substitutions for a sub-pixel stroke", 6.7% apart with every clip off — and it is not a stroke page:
`pdf-model/examples/sub_pixel_width_census` says it states **four** strokes under a device pixel, and
a per-command ink diff puts −3306 of ink on its four 2480 × 2630 sampled floor plans where the
page's whole net gap is −3078. The gap is in the residual of an image reduction and it is raster's;
`doc/QUORRA_FEEDBACK.md` §46 is the ask, with the 8× rung as its control.

## What the movers are

`raster_golden` moved **78 of 974** first pages and every one is `raster only` — display list and
reports byte-identical (trap 37). `pdf-model/examples/own_ink` over the same two builds: **77**
moved in ink, 44 up and 33 down, median **0.0013** levels of 255, **57 of 77 under a hundredth of a
level** and one past a whole level (`standard_fonts.pdf`, −1.15). The seventy-eighth,
`bug1782186.pdf`, moved in placement with its ink unchanged. Twelve were opened and looked at
(trap 1) — `standard_fonts`, `issue20232`, `issue12295`, `issue11473`, `issue4402_reduced`,
`issue16038`, `issue18030`, `knockout_groups_test`, `bug1743245`, `issue14953`, `issue6081`,
`22060_A1_01_Plans` — table grids, technical-drawing frames, hatch swatches, an ECG trace and a
squared notebook page, every one the same picture with its rules where the document put them.

The oracle's verdicts are **identical before and after**: 1960 pages, agrees 1010, contradicted 46,
ambiguous 835.

## What is deliberately not done

- **`sub_pixel_bands` is kept for an axis-aligned rectangle thinner than a pixel**, although the
  general converter would now measure it. The closed form is exact, costs neither a stroker nor a
  path conversion, and lands on the same coverage — and it is what draws `issue15150.pdf`'s
  quarter-pixel mark at **0.251** where the outline route reads 0.5, because `s` closes that
  subpath and the outline is one rectangle traversed twice the same way round.
- **The widened band is kept below one level**, which is the only place it still buys anything: a
  coverage under 1/255 is a mark the raster cannot hold at all, and the alpha is the one place left
  to put it (`expressible_coverage`, ADR 0226).
- **Nothing in `raster/` moved.** The other backend already draws the outline; what it owes is
  §45's winding clamp and §46's reduction residual, both asks rather than changes here.
- **`draw_long_mitres` follows the same boundary**, so a sub-pixel stroke with a mitre §8.4.3.5
  admits and `tiny-skia`'s stroker bevels gets its wedge like any other. It moved no page.

## What it costs

**Less than nothing, on the pages that pay for the construction at all.**
`callgrind_rasterise`, five rasterisations, `RAYON_NUM_THREADS=1`, both arms from one binary so
that trap 16 has nothing to say about which program each figure came from:

| page | before | after | |
|---|---|---|---|
| `standard_fonts.pdf` p1 | 693,524,837 | 564,919,713 | **−18.5%** |
| `issue12295.pdf` p1 | 18,599,280,935 | 8,855,580,711 | **−52.4%** |
| `22060_A1_01_Plans.pdf` p1 | 11,457,013,643 | 11,462,722,256 | +0.05% |

ADR 0268 priced its own construction at +33% on `issue12295.pdf`, whose page one states 65 859
strokes under a device pixel, and that is what comes back: the widening built an outline at the
substituted width *and* a second mark for §8.4.3.3's caps at the alpha their own square implies,
where drawing the stroke the document states builds one outline whose caps the stroker has already
put on it. The plan drawing is the control — four sub-pixel strokes, so nothing to save — and it
does not move.
