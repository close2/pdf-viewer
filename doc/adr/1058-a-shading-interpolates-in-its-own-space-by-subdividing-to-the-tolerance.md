# ADR 1058 — §8.7.4.4's own-space interpolation is a subdivision measured against §10.7.3, asked of a patch once

## Status

Accepted.

## Context

ISO 32000-2 §8.7.4.4 gives each family of colour space a rule about *where* a shading's gradient is
calculated. A device space may be converted "at any time (before or after any interpolation on the
colour values in the shading)". A CIE-based space may not: "all gradient fill calculations shall be
performed in that space. Conversion to device colours shall occur only after all interpolation
calculations have been performed." Nor may a `Separation` or `DeviceN` space: "gradient fill
calculations shall be performed in the designated Separation or DeviceN colour space before
conversion to the alternate space. Thus, nonlinear tint transformation functions shall be
accommodated for an optimal representation of the shading."

`pdf_model::mesh` converted every vertex of a mesh shading to a device colour as it read it,
whatever the space, and handed the rasteriser triangles with a colour at each corner to interpolate
linearly. For a tint transform that is not a straight line that is the wrong picture — the
`tests/shadings.rs` fixture draws 129 where the clause's arithmetic gives 65 — and the ledger's
§8.7.4.4 row had carried the departure since session 43. A display list carries device colours, and
ADR 0028 keeps it that way: no backend evaluates a PDF function or a colour space.

## Decision

**Interpolate in the shading's own space by keeping the stated components through the
subdivision, and convert after it — as many triangles as §10.7.3's tolerance needs.**

1. `mesh::Components` is a third thing a vertex can carry beside a device colour and §8.7.4.5.5's
   parameter. `ColourSpace::interpolates_in` answers the clause's rule for the family — `None` for a
   device space, the space itself for a CIE-based, `Separation` or `DeviceN` one, and the *base* for
   an `Indexed` one, whose values "shall be immediately converted to the base colour space" through
   `ColourSpace::entry_of` before any mixing.
2. A triangle whose corners carry components is emitted where a rasteriser's plane between its
   converted corners is within the tolerance of the conversion of the interpolated components at
   the three edge midpoints and the centroid, and split in four otherwise (`MeshReader::refine`).
   The tolerance is `1 / Colouring::resolution`, the reciprocal of the sample count
   `Ramp::resolution_for` derives from the same `/SM`, so a mesh and a ramp under one graphics
   state answer to one number.
3. Two bounds, `MAX_REFINEMENT_DEPTH` (6) and `REFINED_TRIANGLES` (`MAX_TRIANGLES / 2`), stop a
   subdivision that cannot reach the tolerance, and a mesh so stopped is reported as
   `LimitReached { limit: "max_mesh_refinement" }` — distinct from `max_mesh_triangles`, because the
   document's triangles are all there and what is coarser than asked is the colour between them.
   The half-bound is what keeps the document's own triangles ahead of the ones this crate adds.
4. **A patch is asked the question once, at nine of its grid vertices, before any cell is.** This
   is the part a later round must not undo without the measurement. The first shape converted every
   grid vertex and every triangle's midpoints, and `personwithdog.pdf` — two `DeviceN` tensor meshes
   of some three hundred patches, type 4 tint transforms — went from **92 ms to 2.63 s** at scale 1
   (`examples/open_one`) while **no pixel moved at one or four times**, because the transforms are
   linear and not one triangle was refined. So `Components::emit_patch` converts the four corners
   (which the file's own route converted anyway), the four edge midpoints and the centre, and where
   the five agree with the bilinear mix of the corners to the tolerance the conversion is linear
   across the patch to that tolerance and the grid's colours *are* that mix. The page is at
   **96.8 ms**. A patch that fails converts its 121 grid vertices — the requirement paid in full —
   and `cell_error` asks each cell from the converted grid alone, by second differences, whether a
   rasteriser's plane clears the tolerance; only a cell that cannot be cleared by estimate goes to
   `refine`, which measures.

## Consequences

- `examples/mesh_census` walks `doc/pdf.js` and `doc/corpora` and prints the population the rule
  reaches: three documents, `personwithdog.pdf`, `bug1703683_page2_reduced.pdf` and `178360.pdf`,
  every one with a conversion linear across its patches. **No corpus page on this disk can show
  the change**; the three fixtures in `tests/shadings.rs` are the witnesses, and each fails with the
  conversion moved back before the interpolation. The §8.7.4.4 ledger row says so (trap 8).
- `raster_golden` holds every page, the two `pdf.js` witnesses included: the `Components` route
  emits the same triangles with colours mixed by the same arithmetic, so neither the picture nor
  the display list's digest moves. The first shape, which converted every grid vertex, moved the
  digest of both and no pixel — the same fact seen from the other side.
- What the estimate and the four-point check cannot see is a conversion that turns between two
  samples and back, which §10.7.3's NOTE 1 concedes of a sampled function; the step fixture is the
  case the depth bound catches and reports.

## Alternatives rejected

- **Carry n components per corner in the display list and convert per pixel.** Every backend
  would evaluate colour spaces and PDF functions, which ADR 0028 keeps out of them, and a colour
  space is not a thing a GPU shader can be handed.
- **A per-pixel conversion pass in `pdf-model`.** `doc/todo/13` prices it; it is the transfer
  function's answer and larger than this clause's.
- **Refine every triangle, and accept the cost.** Thirty times slower on a page whose picture did
  not change is principle 2 broken for principle 5's letter, and the patch-level question answers
  the same clause at nine conversions a patch.
- **A coarser internal tolerance for meshes than for ramps.** §10.7.3 permits a device its own
  limits, but two limits under one `/SM` would make the answer depend on the shading type.
