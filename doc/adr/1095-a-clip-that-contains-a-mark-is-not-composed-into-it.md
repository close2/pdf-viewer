# ADR 1095 — A clip that contains a mark is not composed into it

Status: accepted, 2026-09-15. Session 1081. Answers ISO 32000-2 §10.7.4's clipping sentence on the
backend `CLAUDE.md` principle 2 makes the correctness oracle, and closes the two pages ADR 1088
deliberately put on `render-raster`'s differing list — `bug1844576.pdf` and `bug1978317.pdf` — by
moving the oracle onto the geometry rather than by re-stating a clip on the device.

## What the clause says

§10.7.4:

> For clipping, the clipping region consists of the set of pixels that would be included by a fill
> operation. Subsequent painting operations shall affect a region that is the intersection of the
> set of pixels defined by the clipping region with the set of pixels for the region to be painted.

§8.5.4 says the same of the shape — "[t]he effective shape is the intersection of the object's
intrinsic shape with the clipping path". `S ∩ C = S` where `S` lies inside `C`, at every pixel.

## Why `min` was not enough, which is the finding

ADR 0355 made this backend meet a clip by `min` rather than by a product, and `min` has the set
property — *while both sides are measured the same way*. Two things broke that.

**First, a mark that reached no composition at all.** `pdf_render::thinnest_line` is `1 /
max_stretch` through a square root; `pdf_render::band_substitute_width` reaches the same singular
value through a ratio of mapped lengths. Mathematically the second is never under the first — with
`s1 ≥ s2`, `|T u| / |u|` lies in `[s2, s1]` and `|det T|` is `s1 · s2`, so every direction's answer
lies in `[1/s1, 1/s2]` — and in `f32` they parted by a unit in the last place on `bug1844576.pdf`'s
own appearance matrix: **1.0 against 0.9999999**. `render_cpu::draw_sub_pixel_rule` admits a stroke
at or under the first and hands the widening a stroke at or under the second, and
`draw_stroked_outline` declines the same width from the other side, so a `1 w` annotation border
fell between all three to `tiny_skia::PixmapMut::stroke_path` — where a clip *multiplies*. The page
lost **9.0%** of its ink to a `c²` at every boundary pixel of its border.

**Second, two converters measuring one boundary.** With the floor in place the mark reaches
`scan::fill`, and a stroker's ring has a same-wound crossing at its close, so ADR 1082's exact
converter declines it and `tiny-skia` states its boundary pixels **to the nearest quarter**. The
clip beside it is a rectangle, measured by §10.7.4's own closed form (ADR 0476). `min` then cuts the
boundary pixel the quarter rounded *up* and does not restore the one it rounded *down*: 1.2% of the
same page, and the residue is structural rather than a rounding.

## The decision

**A clip whose region contains the mark is not composed into it at all.** `MaskCache::cuts_nothing`
asks that per command, in **page** space, against the rectangle the chain admits
(`DisplayList::clip_admits`, an *inner* bound and the opposite of `clip_bounds`); the mask is still
built, because it is what bands the draw and a mark inside the region is inside its rows. With no
composition there is no arithmetic for two converters to round differently, which is what makes this
a fix rather than a narrowing of the gap. It is the same licence `render_raster::scene::Encoder::
cuts_nothing` takes (ADR 1088), so the two backends now agree here by one rule rather than by luck.

**And the containment is asked about the shape the *document* states, not the one this backend
paints.** That is the half a later round must not reverse. §10.7.4's substitutions widen a mark too
thin to measure so that "no shape ever disappears", and the clause then ranks the two answers where
a clip meets the wider one:

> The area covered by painted pixels shall always be at least as large as the area of the original
> shape.

With `S` inside `C` the intersection leaves `S` whole, so the area owed is `S`'s. Cutting the
substitute back to `C` paints *less* than that — on `bug1978317.pdf` a rule half a device pixel tall
sits in a clip half a device pixel tall, 15 004 times over, and the widened band keeps three quarters
of its ink — which is the forbidden side of the sentence above. So the substitute is drawn whole, and
what it puts outside `C` is bounded by the half device pixel ADR 0268 and ADR 0154 already spend on an
unclipped mark.

A **group** keeps its clip whatever it contains: §8.5.4 gives its result "the union of the shapes of
its constituent objects" and nothing bounds that union without walking the elements.

## What it moved, and the numbers

`render-raster/examples/clip_cost`, page scale, cpu clipped of cpu unclipped:

| page | before | after | raster |
|---|---|---|---|
| `bug1844576.pdf` | 849.31 of 933.38 | **933.38** of 933.38 | 933.09 |
| `bug1978317.pdf` | 13 387.41 of 15 165.68 | **15 165.68** of 15 165.68 | 15 118.43 |
| `issue16473.pdf` | 498.45 of 501.93 | **501.93** of 501.93 | 501.87 |
| `bug1844583.pdf` | 486.46 of 501.76 | **501.76** of 501.76 | 500.93 |

`raster_golden` moved 67 of 974 first pages, every one `raster only`; `own_ink` puts 53 up and 14
down, median +0.017%, largest `bug1978317` +13.28%, every fall under 0.18%. `pdf-transform --test
gate` reads 171.0 pages/s before and 176.1 after, against a floor of 40.

## The gate

`crates/render-cpu/tests/clip_that_cuts_nothing.rs` is `bug1844576.pdf` in miniature — a `1 w`
rectangle stroke inside the `/BBox` its own outline fills, under that document's own appearance
matrix — and holds the clipped reading to the **unclipped** one rather than to the geometry, which
is what `S ∩ C = S` actually says. Its second test holds the other side, that a clip which cuts
still cuts, so a containment test answering `true` for everything fails it; its third holds
§10.7.4's third sentence, that the outline is never drawn short of its own area.

Calibrated by planting each half back off (trap 13): with both off the contained mark reads **11.95
against 16.02**, with only the licence off **16.60 against 16.87**, with both on **16.87 against
16.87**. **The calibration found a defect in the test rather than in the tree**, twice over: the
first draft put every side of the outline on a *quarter* of a device pixel, which both converters
state exactly, and it passed with the rule off; and the floor's own half is invisible here because
the licence subsumes it, so it is pinned separately by
`pdf_render::sub_pixel`'s `the_band_width_is_never_under_the_thinnest_line`, on the witness
transform, which fails the moment the floor is removed.
