# 1236 — A stroke is cut as the outline it marks, and a Bézier at the roots where it crosses

Status: **accepted**.
Context: `crates/pdf-transform/src/redact/paths.rs` (`SubPath`, `crossings`,
`split_at_crossings`, `clip_to_half_plane`, `is_polygonal`, `write_polygons`, `Cut::margin_holds`),
`crates/pdf-transform/src/redact.rs` (`PathObject`, `BuiltSubPath`, `GraphicsState`,
`Walk::paint_path`, `Walk::stroke_outline`, `Walk::cut`, `Walk::set_dash`, `Walk::ext_gstate`,
`split_subpaths`, `STROKE_TOLERANCE`), `crates/pdf-transform/tests/redact.rs`,
`crates/pdf-transform/Cargo.toml`.
Builds: ADR 1195 (the nine-cell cut and its margin proof), ADR 1196 (forms and shared objects),
ADR 1124, ADR 1120 (the provenance fence), `doc/questions/A64`, `doc/questions/A65`.
Amends: nothing; it lifts two refusals ADR 1195 wrote down and narrows a third.
Clauses: ISO 32000-2 §12.5.6.23, §8.5.2.1, §8.5.2.2, §8.5.3, §8.5.3.1, §8.5.3.2, §8.4.2, §8.4.3.2,
§8.4.3.3, §8.4.3.4, §8.4.3.5, §8.4.3.6, §8.6.8 (Table 74), §11.6.4.4, §8.10.1, §7.3.3.

## Two refusals whose reasons were about this build rather than about the geometry

ADR 1195 cut a painted path to the complement of the redaction region and refused two shapes. Both
sentences were true of the code and neither was true of the geometry:

- *a §8.5.2.2 Bézier segment — "the crossing parameter is a root this build does not solve, and
  flattening approximates"*;
- *a stroke — "§8.5.3.2's marks are the path's outline, so cutting the path would place caps and
  joins the producer never wrote"*.

The second sentence contains its own answer. If the marks are the outline, then the thing to cut is
the outline, and cutting it places nothing: the caps and joins are already in it, where the producer
put them.

## The Bézier: the crossing is a root, and it is a root of a cubic in Bernstein form

The cut's only question about a segment is where it crosses a window's boundary line. That boundary
is axis-aligned in the display list's space, so the half-plane's depth is an **affine functional**
of the point; the mapping from the content stream's user space into the display list's is affine
too. Composing the two, the depth along a segment is a polynomial in the parameter whose
**Bernstein** coefficients are the depths of the segment's own control points — degree one for a
line, three for a cubic. Converting to the power basis and solving is a closed form.

The segment is then split at those parameters by de Casteljau, which is exact in arithmetic and
produces sub-curves lying on the source curve: `a_split_piece_lies_on_the_source_curve` checks the
parameterisation lines up, piece at `u` against source at `t·u`. Nothing is flattened and no mark
moves. A segment with no crossing is returned **whole**, so a subpath the cut does not reach comes
out with the producer's own control points bit for bit.

Sutherland–Hodgman generalises to a curved ring without change: pieces on the kept side are emitted
in order, and where a run was dropped the next kept piece is reached by a straight edge whose two
ends both lie on the boundary — the edge the algorithm has always laid along the window.

## The stroke: expand, cut as a fill, and pay the fill in the stroking colour

§8.5.3.2's stroke is the region a pen of the current line width sweeps along the path, closed off by
§8.4.3.3's caps, turned by §8.4.3.4's joins and interrupted by §8.4.3.6's dash pattern. So the
removal is the same geometric cut applied to that region. The dash comes **first**, because it
decides which stretches are marked at all; the expansion takes it in the style and emits each dash
as its own contour.

Three consequences, each with a clause:

- **Painted with `f`, not `S`.** The survivor is a region, and `f` is the operator that paints one.
  §8.5.3's `B` and its three relatives state two marks, so each is cut on its own and written in the
  clause's own order, "[f]ill and then stroke the path".
- **The fill has to be given the stroking colour.** Table 74 pairs each stroking operator with the
  non-stroking one that sets the same thing, so the walk records the producer's own operand
  **bytes** at every `CS`, `SC`, `SCN`, `G`, `RG` and `K` and replays them under the paired operator
  inside a §8.4.2-balanced `q`/`Q`. Bytes rather than numbers: nothing is re-formatted and no colour
  is reinterpreted. The record is saved and restored by `q` and `Q` like the transform beside it,
  and carries into a form the way §8.10.1 says the graphics state does.
- **§11.6.4.4's two alpha constants are not the same constant.** A fill takes `/ca` where a stroke
  takes `/CA`, so an ExtGState that has made them differ makes the substitution visible, and the
  page is refused.

## What is admitted is decided from the expansion's output

An offset of a straight segment, a butt or projecting-square cap and a miter or bevel join are
computed in closed form; a round cap, a round join and the offset of a curved segment are
**approximations of arcs**. Cutting an approximation would replace the producer's marks *outside*
the region with marks this program computed, which is the far side of `CLAUDE.md`'s provenance line
— the thing this whole operation exists not to do.

So `is_polygonal` asks the **output** whether an arc came back, rather than asking the input whether
one should have. Trap 40's shape in reverse: a guard derived from what the library produced cannot
be wrong about what the library produces. `STROKE_TOLERANCE` is stated and its honest description is
that it decides nothing this build admits — it governs exactly the approximations the guard then
refuses, and a later round admitting an arc owes the margin arithmetic for it.

Three further refusals, each narrower than the one it replaces: a **zero line width**, which
§8.4.3.2 states in device pixels and no user-space outline can carry; a stroke this walk has seen no
stroking colour operator for, because §8.6.8's initial value is not something a walk that entered a
form can assert; and the alpha case above.

## The proof, and where it is one step weaker

ADR 1195's proof is pixels at 150 dpi. The stroke meets it exactly:
`a_stroked_path_is_cut_as_the_outline_it_marks` is **byte-identical** outside the region and empty
inside it, which is what an exact polygonal outline buys — and it is evidence the substitution of a
fill for a stroke is invisible in this rasteriser, not merely small.

The curve's is weaker by one eight-bit step and the test says so in its own comment. The geometry is
the producer's own curve restricted to a sub-interval of itself, but the rasteriser flattens a cubic
**adaptively**, and its subdivision of a sub-curve is not the subdivision it gave that stretch of
the whole curve. So the assertion is the property that discriminates a moved mark from a
re-subdivided one: a pixel the original painted whole, or left untouched, is identical; a pixel that
differs had partial coverage and differs by at most one step. A mark that actually moved would turn
a white pixel grey, and that fails here.

## Cost

`kurbo` becomes a dependency of `pdf-transform`. It is not a new dependency of the project —
`render-raster` expands strokes with it — so what is taken is a reading this tree has already made
once rather than a second one. The alternative was writing an offsetting routine here, which would
be a second answer to a question this tree has answered.

The fixtures are hand-built and the round says so rather than implying a corpus ranked any of it:
no corpus document carries a painted path, a form or an image under a redaction region, and the
census stays zero (trap 8).
