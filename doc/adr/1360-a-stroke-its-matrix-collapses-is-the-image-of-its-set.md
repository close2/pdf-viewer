# 1360 — A stroke its matrix collapses is the image of its set, and a line across the axes is a band

Status: accepted. Session 1261. Amends ADR 1348 section 2 (what is restated, and in which space).
Context: ISO 32000-2 §10.7.4, §8.4.3.2 to §8.4.3.6, §8.5.3.2, §8.5.3.3.1, §8.3.4 NOTE 3, §10.7.1;
ADRs 0154, 0482, 1060, 1348; `doc/todo/11` item 8.
Code: `crates/pdf-render/src/collapsed.rs` (`collapsed_by_transform`,
`collapsed_stroke_by_transform`, `ImageLine`), `crates/pdf-render/src/collapsed/stroke_image.rs`,
`crates/pdf-render/src/display_list.rs` (`DisplayList::restate_collapsed_marks`).
Tests: `stroke_image.rs`'s nine closed forms, `collapsed.rs`'s own,
`crates/pdf-model/tests/scan_conversion_collapsed_by_transform.rs`.

## 1. What a stroke under a matrix of rank one is

§8.4.3.2 states the stroke in user space: "stroking a path shall entail painting all points whose
perpendicular distance from the path in user space is less than or equal to half the line width",
with §8.4.3.3's caps, §8.4.3.4's joins, §8.4.3.5's limit and §8.4.3.6's dashes. §10.7.4 then
scan-converts the shape whose "coordinates are mapped into device space". A matrix of rank one
maps the plane onto a line by a linear functional `s(p) = g · p`, so the stroke's image is `s` of
its set. The set is a union of convex parts — each chord's band, each join's wedge or sector, each
cap — and every connected piece of it (a subpath, or a dash) is compact, so its image is one
interval, from the least to the greatest `s` over the piece. A convex polygon's extremes are at
its corners; a disk sector's is `h|g|` where `g`'s own direction lies inside the sector and at its
two radii otherwise. The image is the union of those intervals, and it is stated as the fill of
one flat subpath per interval, which `split_collapsed_fill` draws as §10.7.4's line (trap 2: the
geometry is the shared crate's, no backend's stroker is asked).

ADR 1348 said only a backend's stroker could state this, because it pictured the *outline* being
projected. The outline is not needed: the projection of a convex part is two numbers, and the parts
are the ones ADR 1348 section 1 already names for a folding stroke. The width shows exactly where
the matrix collapses the path itself: a vertical rule under `1 0 0 0 0 50.3 cm` is one point of the
line and five pixels of it at `5 w` — `a_rule_the_matrix_makes_a_point_is_its_width`.

Choices, each where the clauses are silent: curves are flattened in user space to a 256th of a page
unit along the line (ADR 1348's arc figure), with a round sector at a vertex the flattening made; a
dash the path's end cuts to no length is not §8.5.3.2's zero-length dash (an entry of the pattern
that is zero) and marks nothing; a dash crossing a closed subpath's first vertex is one dash joined
there. A pattern past 65 536 dashes or a curve past 1024 chords is refused and stays counted.

## 2. Which matrices, decided exactly

A 2 × 2 matrix of `f32` has rank one when `ad = bc` and it is not zero, and a product of two `f32`
is exact in `f64`, so the test is exact. It replaces ADR 1348's bit-equality of mapped coordinates,
which could only see a line along a page axis, and keeps its property: a matrix whose determinant
cancels only in `f32` (`4605705.pdf`) is refused as the full-rank matrix it is.

## 3. A line across the axes is a band, and a point is ADR 1060's

§10.7.4's rule reaches both: a segment across the grid intersects a staircase of pixels, a point
one. **The line is drawn**: restated in the frame whose `x`-axis is the line, it is the band of one
device pixel `split_collapsed_fill` already draws wherever the grid is turned — the anti-aliased
construction of the row's departure (1), which §10.7.1's NOTE licenses; the staircase is the aliased
rule that departure replaces. Fixture: `1 1 1 1 0 0 cm` carries a rectangle onto the diagonal,
`70√2` of ink within a pixel of it. **The point is not drawn**, and that is ADR 1060's one decision
rather than a new one: a fill whose own subpath is a point already paints nothing, on §8.5.3.3.1's
own "device-dependent and not generally useful", and a mark whose matrix carries it onto a point —
a rank-zero matrix, a fill lying along the matrix's kernel, a stroke of zero width there — is the
same shape. Lifting it means lifting both, in `collapsed.rs`. It stays counted as
`Unsupported::NoninvertibleMatrix`, with a paint no space positions.

## 4. What it moved

Planted away, `a_stroke_its_matrix_flattens_is_its_set_on_the_line` and
`a_rule_the_matrix_makes_a_point_is_its_width` fail with nothing drawn and the matrix reported.
`pdf-model/tests/variable_text.rs`'s singular `Tm` onto `x = y` now draws its band, not nothing.
