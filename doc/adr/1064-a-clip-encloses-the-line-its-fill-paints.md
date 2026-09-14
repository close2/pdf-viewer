# ADR 1064 — A clip encloses the line its fill paints, and the mixed path is where that stops

## Status

Accepted, 2026-09-14. Session 1050. Takes the defect ADR 1060's Consequences recorded and did not
take. Adds `pdf_render::clip_region` (`pdf-render/src/collapsed.rs`) and uses it in all three
backends' clip construction: `render-cpu/src/lib.rs`, `render-gpu/src/scene.rs`,
`render-raster/src/scene.rs`. Tests: `render-cpu/tests/zero_area_clip.rs`,
`render-gpu/tests/headless_gpu.rs::cpu_and_gpu_agree_on_a_clip_with_no_area`, three in
`pdf-render/src/collapsed.rs`. `§N` is ISO 32000-2 and nothing else.

## Context

§10.7.4 defines a clipping region by the fill of the same path —

> For clipping, the clipping region consists of the set of pixels that would be included by a
> fill operation.

— and, two paragraphs earlier, says what the fill of a flat rectangle includes:

> A zero-width or zero-height rectangle paints a line 1 pixel wide.

`pdf_render::split_collapsed_fill` has built that line for a **fill** since session 186 (ADR 0208).
No backend asked it for a **clip**. Measured on `render-cpu` at one, two and three-and-a-half
pixels per unit, `5 20.5 30 0 re f` paints 30, 60 and 106 device pixels, and `5 20.5 30 0 re W n`
over a full-page fill admitted 0, 0 and 0 — the two halves of one definition disagreeing with each
other. The clause attaches no hedge here, unlike §8.5.3.3.1's point, which is why ADR 1060 kept the
degenerate subpath as a choice and left this as a debt.

## Decision

**One function builds the clipping region for every backend**, `pdf_render::clip_region`, beside
the one that builds the fill's marks — a region wider than its own path is a decision no backend
may make alone (trap 2), and the clip and the fill of one path may not answer differently.

1. **Every subpath collapsed**: the region is the marks, scan-converted under the **non-zero** rule
   whatever the operator asked. Two marks that cross are two shapes rather than one winding, and
   the even-odd rule would punch the crossing of a ruled grid out of the region it clips to.
2. **An ordinary path**: unchanged, and unallocated — `Path::collapses` memoises the walk.
3. **A mixed path**, enclosing an area *and* ruling a line: the region §10.7.4 asks for is the
   union of two fills taken under two different rules, and **no backend's clip vocabulary states a
   union**. `vello`'s `push_clip_layer`, raster's `SceneBuilder::clip` and `tiny_skia::Mask` each
   take one path and one rule; a union is expressible in none of them, and a winding or a parity
   cannot be made to hold two independent regions at once. So a mark joins the path only where
   appending it *is* the union: outside the hull of every subpath that encloses an area, where the
   rest of the path has winding zero and crossing count zero and the appended rectangle adds
   exactly itself. Under the even-odd rule a mark meeting another kept mark is dropped for reason
   1's sake.

**A substituted region is scan-converted rather than measured in closed form.** ADR 0476 writes a
rectangular clip's mask from `pdf_render::rectangle_coverage` so that a mark drawn at its exact
area meets a region measured the same way. A mark is a run of *whole* device pixels, where the
closed form and the converter agree arithmetically — but the mark reaches the backend through the
inverse of the placement and back, and at a scale whose reciprocal is not exact in binary that
round trip lands the row a ten-thousandth of a pixel off. `mask_rectangle` writes that sliver into
the neighbouring row; `tiny-skia`'s four sample rows per pixel cannot see it, and the fill's own
marks go through exactly that converter. At scale 3.5 the closed form admitted 212 pixels where the
fill paints 106. The clause defines the region *by* the fill, so the region takes the fill's
converter.

## Consequences

- §10.7.4's row gains **departure (4)**: a mark dropped from a mixed clipping path. Dropping it
  leaves the area the rest of the path encloses, which is the union exactly when the mark lies
  inside it and costs a one-device-pixel line when it does not. It is bounded by construction — a
  clipping path that both encloses an area and rules a line — and it is never worse than what every
  backend did before this ADR, which was to lose every such mark.
- **ADR 1060 is untouched and pinned.** A clipping path that is a single point still admits
  nothing: `Extent::collapse` returns no axis for a subpath collapsed in *both*, so `clip_region`
  declines it and the fill declines it, which is what keeps the two halves of §10.7.4 agreeing in
  the other direction too. `a_clip_that_is_a_single_point_still_admits_nothing` is the control that
  had to keep passing while the planted defect failed the other three (trap 13).
- **`render-raster`'s clip outline now carries a decision about this target's pixel grid**, so it
  consumes the view (ADR 0702) where a region was substituted. raster places a *fill*'s marks per
  viewport from the collapse table resident on the outline; a clip's outline is transient and is
  built here, so the scene it goes into is true at the target it was built for. That is the price
  of the clip and the fill agreeing on a page that states one, and it is paid on no other page.
- The CPU backend's band is widened by one device pixel where a chain step's path collapses
  (`room_for_marks`), because the mark reaches past the bound the band was computed from.
- **The corpus has no witness, and the census that says so is calibrated** (trap 13). Counting at
  the top of `clip_region` reaches 38 267 calls over the 965 corpus first pages `raster_golden`
  draws; counting at the substitution reaches **0**. No first page of the pdf.js corpus states a
  clipping path with a subpath collapsed along exactly one axis, so `raster_golden` holding 974 and
  moving 0 is a control rather than a witness — and a corpus that contains none of a construction
  says nothing about the clause that requires it (trap 8).
