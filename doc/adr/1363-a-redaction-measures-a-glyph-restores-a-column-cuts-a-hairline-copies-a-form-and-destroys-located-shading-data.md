# 1363 — A redaction measures a glyph, restores a column, cuts a hairline, copies a form and destroys located shading data

Status: accepted and **built**.
Context: `crates/pdf-transform/src/redact.rs` (`CompositeCodes`, `Walk::write_run_gap`,
`Walk::cut_hairline`, `split_open`, `Carried`, `mask_state`, `place_located`),
`crates/pdf-transform/src/redact/type3.rs`, `redact/forms.rs`, `redact/located.rs`,
`redact/mesh.rs`, `redact/paths.rs` (`subtract_along`, `write_open`), `redact/shading.rs`,
`crates/pdf-font/src/metrics.rs` (`VerticalDisplacements`, new and public),
`crates/pdf-transform/tests/redact.rs`.
Supersedes: ADR 1351's refusals of a `d0` glyph under an all-zero `/FontBBox`, of a vertical
`CMap`, and of a shading whose colours are data placed in the plane; ADR 1236's refusal of a
zero-width stroke; ADR 1196's refusal of a form drawn twice whose placements the region meets
differently. Amends ADR 1351 section 3's reading that a gradient law holds nothing located (a
radial shading's inner circles can).

§12.5.6.23 asks a processor to "remove all traces of the specified content" and forbids hiding
image data: "clipping or image masks shall not be used to hide that data". Each of the five
refusals round 1257 left is lifted by reading the clause that defines the content.

## 1. A `d0` glyph under an all-zero `/FontBBox` is measured by its own marks

Table 110 withdraws the font box then — "If all four elements of the rectangle are zero, a PDF
processor shall make no assumptions about glyph sizes based on the font bounding box" — and §9.6.4
leaves the description itself: "glyphs shall be defined by streams of PDF graphics operators". So
the description is run through the same interpreter the page was drawn by, on a probe page carrying
it as its content under §7.8.3's resources for a glyph, the display list's command bounds
(`Command::device_bounds`, which errs outward) are taken and carried back into glyph space, and the
code is tested against that box as ADR 1351 tests a declared one. The description inherits every
graphics state parameter but the CTM "from the graphics state at the point of invocation of the
text-showing operator"; of those only the line width and miter limit move a mark's extent, and they
are stated in front of the description. A description the interpreter could not draw in full is
refused by name: a part undrawn is a part unmeasured. No accessor in `pdf-model` was needed.

## 2. A vertical `CMap`'s removed code restores its `w1`

In writing mode 1 §9.4.4's displacement is `ty`, and the placed quadrilateral does not carry it: it
is the glyph's horizontal box moved back by its position vector. So the advance is read from what
the interpreter moved the pen by — the descendant's `/W2`, else `/DW2`'s second number (§9.7.4.3,
"the CIDFont shall define the vertical displacement for each glyph") — through a public
`pdf_font::VerticalDisplacements`, the same `Vertical` reading the loader places glyphs with. The
`TJ` number is "subtracted from the current horizontal or vertical coordinate, depending on the
writing mode" (§9.4.3), so with `Tc` and `Tw` zero (spacing in force still refuses) it is minus the
removed codes' summed `w1y`; `Th` scales `tx` alone. The fixture carries §9.7.4.3's EXAMPLE 2 and 3.

## 3. A zero-width stroke is cut as a path

§8.4.3.2: "A line width of 0 shall denote the thinnest line that can be rendered at device
resolution: 1 device pixel wide." No outline in user space states that, but none is needed: the
marks lie along the path at whatever resolution draws it. The path is split where it crosses each
widened region edge (ADR 1236's roots, so a curve stays a curve), the pieces whose midpoints lie in
the region go, and the rest are stroked again by `S` at the same width. A dash pattern is applied
first and the dashes stroked solid inside `q`/`Q`, since a pattern restarted at each piece would
mark different stretches.

## 4. A form met differently at two placements is given a copy per edit

The first walk of a page records every placement of every form object and the edited stream it
asked for (or none). A form whose placements disagree is split, and a second walk writes each
placement its own: a placement the region missed keeps the producer's form under the producer's
name; an edited placement draws a copy — the producer's dictionary, its own stream — under a fresh
`/XObject` entry its enclosing stream's resources gain, its `Do` rewritten; where no placement was
missed, the first edit takes the form's own place, so the producer's stream is not left behind the
copies. A soft mask's group placement is given a fresh `/ExtGState` entry restating the producer's
graphics state with `/SMask /G` the copy. Equal edits share a copy. Inlining at each `Do` was
rejected: a form is also its `/BBox`, `/Group` and resource names, and restating those inline is a
rewrite of structure the producer chose. Copies are bounded by `forms::MAX_COPIES` (64) per page, a
resource bound refused by name. The first walk's silent over-removal — an unedited placement drawing
the edited stream — is closed by the same record.

## 5. A shading's data are destroyed where they are located

A value is located wherever it colours the plane; a clip does not move it, as a clip does not move
an image's sample. Read per type:

- **Axial**: a parameter's colour lies on a line that "extends indefinitely perpendicular to that
  axis" (§8.7.4.5.3), so nothing is located in the region alone, whatever the function; the clip
  cut is the whole removal. The sampled-axial refusal was a misreading and is gone.
- **Radial**: "as if an infinite number of such circles are painted in turn" (§8.7.4.5.4), each
  opaque over the last. A parameter is the region's alone where its circle lies wholly inside a
  region box: the circle's image box against the box is four affine inequalities in `s`. `/Extend`
  circles carry the end values too, so an end whose extension leaves the region is kept.
- **Function-based**: a sampled 2-in function's sample is located over the cells it is interpolated
  across, carried by `/Matrix` and the placement.
- **Meshes**: triangles are cut exactly — Gouraud interpolation is affine over a triangle, so each
  surviving convex piece is fanned with its new vertices coloured by the source triangle's own law;
  patches are split by de Casteljau and a sub-patch whose control box meets the region is dropped,
  down to `LEAF_EXTENT` (a quarter point), the sub-patch being the unit of removal as the code is
  for text (ADR 1124's bounding-box intersection). Written back at 32-bit coordinates and 16-bit
  components under the producer's `/Decode`, with ADR 1195's margin checked; a lattice becomes a
  free-form mesh and a Coons patch a tensor patch — "[t]he Coons patch (Type 6) is actually a
  special case of the tensor-product patch (Type 7)" — because a lattice cannot state a hole and a
  Coons sub-patch's interior need not be the Coons formula's. A mesh's `/Function` parameter is
  located on the pieces that carry it.

What is destroyed follows what a function is. A sampled function's samples are zeroed (ADR 1124's
constant) where every parameter they are interpolated over is the region's alone; `/Order` 3 is a
cubic spline, which is not local, so its samples are cleared only where every parameter is. An
exponential function is determined by any two of its values outside the region, so it is zeroed
only where every parameter it serves is the region's. A stitching function is taken piece by piece.
**A calculator function serving a region-only value is refused by name, and that is a decision**:
its text is not divided by the parameters it serves, no finite re-expression holds its values
exactly (a codec image re-expressed as samples is exact; a program is not), and a program rewritten
to omit the region's values would be one this build authored. The destroyed shading or pattern is
written afresh for the page and the stream's resource entry names it; the producer's object stays
for whatever else names it. Two placements naming one entry take the union, as two placements of
one image do (ADR 1277).

## What the row is after this

Every class the brief named is built, or refused on a decided ground (calculator functions; the
copy bound; damaged or non-conforming input). The overlay stays A64's decided departure. The row
stays `partial` because the codec residue in `doc/todo/64` — a matte its re-expression cannot keep,
a `JPXDecode` whose components disagree on a depth above eight, a decode shape no fresh raster
holds — is a debt rather than a decision: each is a re-expression not yet built.
