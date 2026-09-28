# 1348 — A stroke is the set within half its width, and a fill its matrix flattens is its line

Status: accepted. Session 1255.
Context: ISO 32000-2 §8.4.3.2, §8.4.3.3, §8.4.3.4, §8.4.3.5, §10.7.4, §10.7.1, §8.3.4 NOTE 3,
§8.5.3.3.1, §8.5.4; ADRs 0355, 0482, 0492, 1082, 1341, 1347; `doc/todo/11` items 4 and 8;
`doc/QUORRA_FEEDBACK.md` section 54.
Code: `crates/render-cpu/src/lib.rs` (`folding_stroke_outline`, `Pieces`, `curves_may_fold`),
`crates/render-cpu/src/scan.rs` (`Exact::Pieces`, `fill_pieces`),
`crates/pdf-render/src/collapsed.rs` (`collapsed_by_transform`),
`crates/pdf-render/src/display_list.rs` (`DisplayList::restate_collapsed_fills`),
`crates/pdf-model/src/content.rs` (`finished`).
Tests: `crates/render-cpu/tests/stroke_wider_than_its_curve.rs`,
`crates/render-cpu/tests/image_clip_intersection.rs`,
`crates/pdf-model/tests/scan_conversion_collapsed_by_transform.rs`.

## 1. A curve stroked wider than it bends (§8.4.3.2)

§8.4.3.2 defines the stroke as a set: "stroking a path shall entail painting all points whose
perpendicular distance from the path in user space is less than or equal to half the line width".
Round 1258 found a radius-5 circle at `30 w` drawn as a ring with a hole of radius 10 on this
backend: `tiny-skia`'s stroker offsets a closed curve's inside as a second contour, reversed, and
once the half-width passes the radius of curvature that contour turns over and cancels the outer
one. Handing the stroker the curve's chords was tried and fails the same way once the chords are
fine enough: two chords meeting nearly straight are joined as one line (`SCALAR_NEARLY_ZERO` in
its angle test), with no pivot on the inner side — the hole returns at 552 chords to the circle.
Measured, the stroker's own curves also overshoot a thin ring by 0.3% (756.5 against 754.0).

So where a curve may bend more tightly than the half-width, nothing is offset: the path is
flattened (§10.7.4: "curves have been flattened to sequences of straight lines") and the stroke is
the union of convex pieces, all wound one way — each chord's band, reaching a sixteenth of the
flatness past its ends so that neighbours overlap rather than abut (abutting pieces left a
sixteenth out of interior pixels); at each vertex the join on its outer side, §8.4.3.4's at a
stated vertex with §8.4.3.5's limit and a round join's sector at a vertex the flattening made;
§8.4.3.3's cap at each open end. The test for "may bend" is sufficient: a radius of curvature is at
least `|B′|² / |B″|`, bounded by the hodograph's control polygon, and a curve failing it is halved
six times before it is taken to fold.

**Measured by the library's converter, and that is a cost decision.** `crate::area` settles an
overlap exactly (ADR 1341), but these pieces overlap almost everywhere and its walk is quadratic
in them: `issue14415.pdf`, whose one-point strokes fold at 175 cusps per draw, went to 20.5G
instructions. `scan::Exact::Pieces` sends them to `tiny-skia`'s supersampled converter, which
applies the non-zero rule per sample and is linear in edges: a sixteenth of a pixel, departure
(1)'s quantum where ADR 1082 does not reach, and +55M instructions on that page. The chords lie
within a sixty-fourth of a pixel of their curve (a quarter of that quantum) and turn by at most
what holds a band's rim within the same; the arcs of round joins and caps within a 256th.

The fixtures, against the clause's distance set computed from the control points alone (`numpy`,
a 1/64-unit grid): the circle 1256.03 against 1256.72, a 6×3 ellipse at `14 w` 414.12 against
413.99, an open arch at `16 w` with round caps 566.31 against 565.50. Planted away the circle and
the ellipse have holes at their centres and the arch reads 566.88. `raster_golden` moved 35 first
pages (26 by this construction beyond ADR 1347's one-level pixels): stroke ends that lost a
stroker's spur (`pr12564.pdf`), ink annotations and hand-drawn curves (`inks.pdf`,
`issue19360.pdf`, `rotated_ink.pdf`), every one `raster only`. `render-raster --test corpus`
946 agree / 5 differ before and after. The graphics backend's own defect with folding joins is
`doc/QUORRA_FEEDBACK.md` section 54.

## 2. A fill its matrix carries onto a line (`doc/todo/11` item 8)

§10.7.4's rule is stated for a shape whose "coordinates are mapped into device space", and it
says what such a shape gets: "A shape shall be scan-converted by painting any pixel whose half-open
square region intersects the shape, no matter how small the intersection is", with NOTE 1, "a
filling region is considered to intersect every pixel through which its boundary passes, even if
the interior of the filling region is empty". §8.3.4's third NOTE, "can result in unpredictable
behaviour", is informative and states no behaviour, so it cannot stand against that. The warrant
the item asked for is therefore the clause's own `shall`.

`pdf_render::collapsed_by_transform` restates a fill whose matrix has no inverse and carries every
point onto one line along a page axis as that line in page space, under the identity, and
`DisplayList::restate_collapsed_fills` applies it once, in `pdf_model`'s finished list — so all
three backends draw it by `split_collapsed_fill`'s existing construction (trap 2), and the report
counts only what is still refused. Exact equality of the mapped coordinates is the test, as
`collapsed.rs`'s own is, which keeps `4605705.pdf`'s full-rank matrix — determinant zero only in
the arithmetic — refused rather than restated as the enormous shape it is. The fixture: `80 × 40`
under `1 0 0 0 0 50.3 cm` is device row 49, columns 10 to 89, eighty pixels of ink exactly;
planted away it is refused and counted.

Still refused and counted, each for a stated reason: a **stroke**, whose width is stated in the
space the matrix collapsed, so its image on the line is the projection of the stroke's own outline
— which only a backend's stroker produces, and trap 2 forbids one backend deciding it; a **point**
(§8.5.3.3.1, recorded as a departure in `collapsed.rs`); a **line across the axes**, which
`collapsed.rs` states as its own absence for a path flat in its own space; and a **paint** that is
not a solid colour, which a space with no inverse cannot position. An image under such a matrix
paints nothing and that is §10.7.4's own image rule — "only those pixels whose centres lie within
the region shall be painted", and a region of no area holds no centre.

## 3. The clip product at an image's edge and in a group (`doc/todo/11` item 4)

**An image's edge is paid on this backend**: `draw_image` fills the unit square
through `scan::fill`, whose composable clip meets the mark's coverage by `min`. The item's
"`draw_pixmap`'s" was stale. `image_clip_intersection.rs` is the witness it lacked: an image
edge covering 0.502 of its column reads 0.502 under one, two and three restatements of a clip that
shares that edge and cuts the image's other side; planted as a product it reads 0.255.

**A group whose opacity is below 1.0, and a non-isolated group, are a documented choice under
§10.7.1's NOTE, not a debt.** The clause's clipping paragraph intersects *sets of pixels*; under
the clause's own aliased rule a clip and a mark are each 0 or 1 at a pixel and the product *is* the
intersection. The two part only because departure (1) gives a boundary pixel a fraction, and the
clause states no composition of fractions — "The specifics of the scan conversion algorithm are not
defined as part of PDF. Different implementations can perform scan conversion in different ways".
`min` is this tree's choice where the mark's shape is at hand, because it is exact for coincident
and nested boundaries (ADR 0355); a group buffer holds alpha, shape times opacity, and where
opacity is not 1.0 the shape is not recoverable from it, so the product is the choice there. What
would replace it is a second render of the group's content as shape (ADR 0492's own pricing); no
document has been shown to need it.
