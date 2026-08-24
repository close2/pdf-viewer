# 0583 — Several rectangles are one object, and §11.6.2 is what says so

**Status.** Accepted. Pays the reachable half of `doc/todo/11` item 7's remainder and removes its
stated dependency on item 5.

## Context

ADR 0476 gave an axis-aligned rectangle the coverage §10.7.4's own definition of a pixel implies —
the product of two one-dimensional overlaps, exactly, at every placement — in place of
`tiny-skia`'s supersampled quarter. `doc/todo/11` item 7 then listed what still carries the
quantum, and put one entry on that list under a dependency:

> The last of the list is deliberate and is item 5's subject — two rectangles drawn as two marks
> composite by §11.3.7.3's union where one scan conversion accumulates them, so taking them one at
> a time would trade this defect for a worse one along every seam. A round that wants it needs the
> seam answered first.

`pdf_render::edge`'s own module comment said the same, and both had said it since the
six-hundred-and-forty-sixth session.

## The clause the dependency was addressed to is the wrong one

§11.3.7.3's union is what the standard says to do with **two objects**. A path's subpaths are not
two objects, and §11.6.2 is about exactly that case:

> Single graphics objects, as defined in 8.2, "Graphics objects", shall be treated as elementary
> objects for transparency compositing purposes … That is, all of a given object shall be
> considered to be one element of a transparency stack. Portions of an object shall not be
> composited with one another, even if they are described in a way that would seem to cause
> overlaps (such as a self-intersecting path, combined fill and stroke of a path, or a shading
> pattern containing an overlap or fold-over).

So the construction item 7 was declining to build is **forbidden by a `shall`**, not weighed
against a seam. There was never a trade to make: the seam item 5 measures cannot arise between two
subpaths of one `f`, because compositing them at all is what the clause rules out.

What the clause leaves open is where the portions land, and that is a question about pixels rather
than about compositing. It splits in two:

- **No device pixel receives two portions.** Then drawing them one at a time composites each with
  the backdrop and none of them with each other, which is what the clause asks for, and each is
  measured by §10.7.4's closed form.
- **Some pixel receives two.** Then drawing them separately would composite two portions in that
  pixel — the forbidden thing — so the mark stays on the one supersampled conversion that
  accumulates the whole path, which already honours §11.6.2 and only measures it to a quarter.

The condition is the clause's own arithmetic and not a threshold anybody chose: §10.7.4 identifies
a pixel by flooring a point, so a portion's footprint is a half-open range of whole pixels and two
portions meeting exactly on a pixel boundary do not share one.

## The census, before the code

`crates/pdf-model/examples/rectangular_path_census` — trap 14's rule that the census belongs before
the construction — over first pages at scale 1:

```text
                                     pdf.js   format-corpus   pdfbox   pdf20   differences
  fill commands                      223 545        132 139   60 562     407         2 778
  one rectangle (exact since 0476)    12 987          4 430    2 817      30           140
  several, no shared pixel             3 419          2 006    2 047      31           110
  several, sharing a pixel               505            166       87       0             0
  a rectangular subpath, declined      3 084          1 151      851       0            55
```

**Seven eighths of the multi-rectangle population needs no coverage buffer at all**, over 151 pages
of 151 documents in the pdf.js corpus alone, and the widest path any of them states is 16
rectangles. The declined column is overlapping rectangles, subpaths that are not rectangles, and
paths above the budget together; overlapping ones are declined on the clause's own grounds, since
§11.6.2's sentence names that case and the two fill rules answer it differently.

## What was built

**`pdf_render::device_rectangles`**, returning `DeviceRectangles::One` or `::Several` — the
decomposition of a path into disjoint device rectangles under an axis-preserving transform — and
`DeviceRectangles::share_a_device_pixel`, the clause's condition above.
`pdf_render::device_rectangle` is now the first variant with the second declined, so **one walk
answers both questions**. That is measured rather than tidy: keeping the two as separate entry
points cost **+0.18%** of the rasteriser on a page of text, which is a second walk over every fill
the first one declined, against +0.07% for the variant.

Two properties are stated rather than assumed, and both follow from the interiors being disjoint:
**their areas add**, so a pixel's coverage is the sum of the portions' and needs no union anywhere;
and **the fill rule stops mattering**, because every point lies in at most one of them and the
non-zero and even-odd rules select one set.

`render_cpu::rectangular_mark` asks both questions and hands the answer to `scan::Exact`, which
replaces the single `Option<Rect>` the backend carried before. Every consumer of the closed form
takes it: `scan::fill_rectangles` for a mark, `scan::mask_fill` for a **clipping region's** mask —
because §10.7.4 says the region "consists of the set of pixels that would be included by a fill
operation", and ADR 0476's lesson is that measuring only one of the two breaks the same paragraph's
set identity `S ∩ C = S`.

**The decision is `pdf-render`'s, which is trap 2 and not a preference.** Both halves are
statements about the device pixel grid, and `render-quorra` already resolves a whole path in one
analytic conversion; a backend that decided on its own when to split a mark into several would be
deciding what neither backend had chosen.

## What it cost, and what it bought

`callgrind_rasterise`, `RAYON_NUM_THREADS=1`, twenty rasterisations, both arms built in one sitting
and the "before" arm built from the unmodified sources rather than by disabling the branch — which
matters here, because an `if true { return }` lets the optimiser delete the new function entirely
and reads 0.1% low:

```text
  ISO 32000-2 p101 (text, no multi-rectangle fill)
                                     5,384,472,180 -> 5,388,457,698   +0.074%
  colors.pdf p1 (ADR 0476's witness)       521,681 ->       521,670   -0.002%
  issue840.pdf p1   (427 such fills) 5,420,592,984 -> 5,417,497,850   -0.057%
  issue1350.pdf p1  (142)            2,975,303,242 -> 2,964,226,161   -0.372%
  issue13447.pdf p1 (289)            6,769,221,234 -> 6,733,313,629   -0.531%
```

Cheaper wherever the population is, because a multi-rectangle path stops being a supersampled path
fill; **+0.074% where there is none**, which is the enum returned by value and the branch per
subpath, and is the honest cost of the construction rather than a rounding.

What it bought, from `edge_coverage.rs` run against the unfixed tree (trap 13, both new scenes
planted and confirmed failing first): a three-rectangle path whose edges fall 0.05 of a pixel
across painted **nothing** at those edges — 0.05 of coverage missing, §10.7.4's "the area covered
by painted pixels shall always be at least as large as the area of the original shape" failing at
the same bottom end ADR 0476 fixed for one rectangle — and the same two rectangles stated as a
**clipping region** were 0.1097 of a pixel out at their worst, 28 levels of 255.

## The blast radius, which is larger than the item implied

`raster_digest` over the pdf.js corpus: **135 of 974 first pages move pixels**, at 0.17% to 0.36% of
their bytes and a worst channel difference of 45 to 58 levels — the quarter being replaced by the
area, in the direction §10.7.4 states.

**And a share of it is text**, which item 7 did not predict and which is correct. A glyph is a
`Command::Fill` of its outline, so a stem that is one axis-aligned rectangle has had the exact
coverage since ADR 0476 and a glyph whose outline is *two* — an `i`, a `"`, an `=` — has it now.
`bug1671312_ArialNarrow.pdf`'s page is nineteen fills of which three are such glyphs. §10.7.4's last
sentence permits a glyph to be scan-converted by the font rasteriser's own algorithm; it does not
require a *worse* measurement of the same shape, and the outline this backend fills is a path like
any other.

## What is left, with its price

**The sharing half** — 505 fills of 3924 in the pdf.js corpus, and the case where two portions of
one object fall in one device pixel. Its construction is not the conflation-free rasteriser item 5
prices: it is one coverage buffer per mark, the pieces' areas **summed** into it and the paint
blitted once through the result, which is exactly the shape `scan::intersected` already has for a
clip (ADR 0355) and would cost what that cost — about +1.2% of the rasteriser on the pages that use
it, plus the reuse of one scratch mask that ADR 0355 found was the difference between +1.2% and
+54%. It is left because seven eighths of the population does not need it and because a round that
takes it should take `scan::intersected`'s clip composition with it rather than growing a second
buffer beside it.

**Overlapping rectangles** stay declined, and that one is the clause's rather than a budget: the
decomposition would have to carry a winding number per cell, which is a scan converter of our own
by another name.
