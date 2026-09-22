# ADR 1217 — A patch's fineness is a question in device pixels, so the patch travels

## Status

Accepted. Supersedes the residue ADR 0919 recorded for §8.7.4.5.7 and §8.7.4.5.8 — "a
tessellation derived from §10.7.3" — and amends ADR 1058, whose colour half stands unchanged.

## Context

ISO 32000-2 §8.7.4.5.7 defines a Coons patch's geometry as a mapping rather than as a polygon:

> Coordinates are mapped from the unit square into a four-sided patch whose sides are not
> necessarily linear

and says the same of what a reader sees — "NOTE The patch is a control surface rather than a
painting geometry." §8.7.4.5.8 adds sixteen control points and nothing else: it is "identical to
Type 6, except that they are based on a bicubic tensor-product patch defined by 16 control
points".

This tree evaluated that surface on an 11×11 grid, `mesh::PATCH_STEPS`, for every patch of every
document at every magnification. Session 945 priced the departure — a mean 0.0511 of 255 on the
corpus's worst mesh page at its own scale, a worst 32×32 tile of 10.18 at four times, because the
error is a seam at each patch's boundary whose *width* does not shrink as the pixels arrive — and
session 1084 sharpened what it was: ADR 1058 derives the *colour* half from §10.7.3 already, so
what `PATCH_STEPS` fixes is the **geometry**, whose tolerance §10.7.2 states "in device pixels".

§10.7.3's NOTE 2 is the clause that separates the two, and it is the whole argument for this ADR:

> The effect of the smoothness tolerance is similar to that of the flatness tolerance. However,
> that flatness is measured in device-dependent units of pixel width, whereas smoothness is
> measured as a fraction of colour component range.

`pdf-model` has the second and cannot have the first. A display list is re-rasterised at any
magnification without being interpreted again, `shading::Cache` caches a built shading across
transforms, and `TargetSpec::for_page` chooses the raster's scale afterwards. So a fineness
decided where the shading is read is a fineness decided before the question can be asked.

## Decision

**The patch travels, and the backend derives the fineness** — the same answer
`ShadingKind::Sampled` already gives §8.7.4.5.2's function, and for the same reason.

1. `pdf_render::SurfacePatch` carries the 4×4 control net and `PatchCorners` — four colours, or
   four of §8.7.4.5.5's parameters — and `PatchMesh` carries the patches beside §10.7.3's
   tolerance under the graphics state the shading was read in. `ShadingKind::Mesh` gains
   `patches`, and `triangles` is empty exactly where it is `Some`, so that §8.7.4.5.7's "[i]f one
   patch overlaps another, the patch that appears later in the data stream shall paint over the
   earlier one" is the order of one sequence.
2. `MeshRaster::build` — which all three backends already reach, so there is one derivation and
   not three — tessellates the patches under the device transform it is handed.
   `SurfacePatch::steps` takes the larger of two answers:
   - **the silhouette**, from the largest second difference of the device net's rows and columns.
     A cubic Bézier is within three quarters of that of its own chord and `n` uniform pieces
     divide it by `n²`; Bernstein weights are non-negative and sum to one, so the largest second
     difference over the four rows bounds every isoparametric curve in that direction. The
     tolerance is **half a device pixel**: §10.7.2's unit, tighter than Table 52's initial 1.0 in
     the direction its NOTE 1 recommends, and half because `Triangle::paint` decides a pixel by
     its centre.
   - **the colour**, from the bilinear cross term of the four corners against §10.7.3's
     tolerance. §8.7.4.5.7 fills a patch by bilinear interpolation and a rasteriser draws each
     cell as two *linear* triangles; the two differ by at most a quarter of that cross term over
     `n_u · n_v`.
3. **A patch is deferred only where the quantity interpolated is one a backend holds.** A device
   colour and §8.7.4.5.5's parameter both are. Components in a space §8.7.4.4 requires the
   gradient be calculated in are not, and ADR 1058's per-patch question decides: where the
   conversion is linear across the patch to §10.7.3's tolerance the four converted corners *are*
   the patch's colour and it travels; where it is not, that mesh is tessellated and converted in
   `pdf-colour` as before. All or none per mesh, so that two producers never decide one paint
   order.
4. **`mesh::MAX_PATCHES` is a bound of its own.** The patch budget was `MAX_TRIANGLES` divided by
   a fineness that has nothing to do with the document; it is now 16 384 patches, fifty times the
   largest mesh any corpus here paints with, and a mesh it stops is reported as
   `max_mesh_patches`. `Shaded::truncated` carries the bound's *name* rather than a flag, because
   two bounds counting two things cannot share one report. The device's own budget,
   `MAX_PATCH_TRIANGLES`, is shared **equally** between a mesh's patches, so a mesh past it is
   drawn coarsely everywhere rather than finely at the front and dropped at the back.

## Consequences

- `raster_golden`: ten of 974 first pages move. **Six are the type 6 and type 7 pages** —
  `bug1703683_page2_reduced`, `coons-allflags-withfunction`, `issue13520`, `issue18816`,
  `personwithdog`, `tensor-allflags-withfunction` — and four moved their display-list *digest*
  alone (`issue17848`, `issue2948`, `issue6231_1`, `mesh_shading_empty`), because that digest is
  the list's `Debug` and the `Mesh` variant gained a field; those four are triangle meshes and
  their rasters are unchanged. Checked rather than assumed: each of the ten was interpreted and
  its mesh commands counted.
- The oracle over those six, run both ways in one sitting: **1 agrees, 5 ambiguous, 0
  contradicted**, before and after. On four of the five ambiguous pages our distance to the
  nearest reference rose slightly (`coons-allflags-withfunction` 0.19 → 0.28, `tensor` 0.34 →
  0.38, `personwithdog` 1.18 → 1.37, `issue13520` 1.89 → 1.90 in three measures), every one of
  them far inside the spread between the closest two references themselves (1.08, 1.11, 1.79,
  12.62). Principle 5: the references are evidence about our reading and the derivation is the
  clause's.
- **Cost, callgrind, one sitting, same binary with the fineness forced back to ten**:
  `personwithdog.pdf` page 1 rasterised three times, **2 463 460 526 → 804 415 600** instructions.
  A patch smaller than a few pixels now costs the two triangles it needs.
- `viewer-confined` carries patches on the wire as patches. A fineness chosen on the interpreter's
  side of that channel would be exactly what the display list is written not to carry.
- `test_scenes::patch_mesh` is the cross-backend witness — two patches with boundaries far off
  their chords and corner colours at the extremes of the mix — and all three backends agree on it.

## Alternatives rejected

- **Raise `PATCH_STEPS`.** It is not a derivation at any value, and the old patch budget fell as
  its square: session 945 measured the collision at a fineness of 21.
- **Derive the fineness in `pdf-colour` from the transform it is handed.** That transform reaches
  the *display list's* space, not the device's; the answer would be wrong by the magnification,
  which is the whole of what §10.7.2 measures in.
- **Carry components per corner and let the backend convert.** ADR 0028 keeps colour spaces and
  PDF functions out of the backends, and ADR 1058 rejected the same thing one clause over.
