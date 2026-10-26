# 2330 — A point is the pixel under it, and a line across the axes is its band

Status: accepted for integration. Session 1748, 2026-10-25, the clause slot of batch one hundred and
fourteen. Clause: ISO 32000-2 §10.7.4, with §8.5.3.3.1 and §8.3.4. Supersedes ADR 1060's decision
for the fill and the clip, and ADR 1360's for a mark a matrix of rank zero collapses; ADR 0154's
reading of the point is withdrawn, its reading of the line kept. Context: ADRs 0154, 1060, 1348,
1360, 1622, 2331.

## 1. Does a zero-area intersection intersect?

§10.7.4: "A shape shall be scan-converted by painting any pixel whose half-open square region
intersects the shape, no matter how small the intersection is." A point lying in a pixel meets it in
that point, so the intersection is not empty; the clause's NOTE 1 reads *intersects* that way for
exactly this case — "a filling region is considered to intersect every pixel through which its
boundary passes, even if the interior of the filling region is empty" — and §8.5.3.3.1 names the
pixel: a degenerate subpath "shall be considered to enclose the single device pixel lying under that
point". So the answer is yes, and the pixel is owed.

ADR 1060 kept the point unpainted on the sentence that follows, "the result is device-dependent and
not generally useful". That sentence has no modal verb. It says which pixel a point lands in depends
on the device's grid, and that a producer gains little by writing one; it permits nothing. ADR 1622
keeps `departed` for a `shall` decided against *and priced*, and nothing priced this one: ADR 1060
measured that no pixel was painted, not what painting it would cost. ADR 0154 had already used the
point's `shall` as the warrant for painting a zero-height rectangle — the sentence true for the
argument and false for the drawing.

## 2. What is built

`pdf_render::collapsed` classifies a subpath that encloses no area three ways (`Collapse`):

- **along a path axis** — unchanged, ADRs 0154, 0208;
- **at a point** — the device pixel `floor(x), floor(y)` under any invertible placement, its corners
  carried back through the inverse (`append_pixel_under`);
- **on a line across the axes** — which the module had named as "not covered": the band of one device
  pixel along the line between its two furthest points, built in device space and carried back
  (`append_band_across`), the construction ADR 1360 gives a line across the grid. Collinearity is
  decided exactly (`on_one_line`): each difference of `f32` coordinates checked exact in `f64` by
  Knuth's two-sum, each product compared with its `mul_add` rounding error, so a sliver with an area
  is never called a line. A cubic is on a line exactly when its control points are.

A matrix of rank zero carries every point onto `(e, f)`, and one of rank one may carry a particular
path onto one point; `collapsed_by_transform` and `collapsed_stroke_by_transform` restate such a mark
as that point in page space, and the fill paints its pixel. A stroke whose set is empty — a
degenerate subpath under butt caps — stays nothing. `Unsupported::NoninvertibleMatrix` now counts
only a paint no space positions. The clip follows the fill, as ADR 1060 required of any reversal:
`clip_region` asks the same split, so `10.5 20.25 0 0 re W n` admits one pixel. `raster-gpu`'s
collapse table and encode mirror all of it (their ADRs 0085, 0086).

## 3. A glyph's contour at one point is the font's

The first walk of the six arms with §2 alone moved 33 first pages at 1x, and a census of the fills it
drew named the population (trap 11): 31 of the 33 were glyph outlines carrying a contour at one
point — a move and a closepath with nothing between, or a curve of no length — inside ordinary
letters of their fonts, `issue9972-1.pdf` 746 of them, each now a speck beside its letter. That is
not a degenerate subpath a content stream constructs. §10.7.4's last paragraph leaves glyphs to a
font's own algorithm — "Scan conversion of character glyphs may be performed by a different algorithm
from the preceding one" — and a font program's contour that names one point states no edge, which a
font rasteriser fills nothing of. So `pdf-font`'s `build_outline` drops such contours
(`without_point_contours`), and a glyph left with none is the blank glyph it already answers `None`
for; a contour with an extent keeps §10.7.4's marks, as an axis-flat one has since ADR 0154. The two
pages left are producers' own: `issue6961.pdf`'s zero-size polygons under a rotation, one dark pixel
at its page's bottom edge, and `bug1721218_reduced.pdf`'s two short strokes stated as fills, a thin
divider now drawn beside a logo. Each was rendered and looked at (trap 1).

## 4. What is decided against

A subpath whose boundary retraces itself off any one line — out along two sides of a triangle and
back — encloses no area under either rule and still marks nothing. Recognising it exactly is a
cancellation of directed edges over the whole subpath, the winding integral's own question, asked of
every subpath that is not on a line; no document this tree holds writes one. It is §10.7.4's row's
departure, with that cost.

## 5. Evidence

`pdf-model/tests/scan_conversion_shapes_without_area.rs` (six tests run red on HEAD, two controls
green on both), `raster-gpu/tests/collapsed_fills.rs` (two, each asserting ink where HEAD's table
held no entry), the unit tests in `collapsed.rs` and `stroke_image.rs`, and
`render-cpu/tests/window_on_the_page_grid.rs` holding a window's points and bands to the page's.
Eight tests pinned the old answer. Seven now hold the new one — `collapsed.rs`'s point,
`stroke_image.rs`'s two, `zero_area_clip.rs`'s point clip, `variable_text.rs`'s glyphs under an
all-zero text matrix, `raster-gpu`'s point and its `no_ink.rs` witness of a fill with no shape, now a
subpath retracing itself off a line, §4's departure — and `scan_conversion_collapsed_by_transform.rs`'s
refusal is removed, its subject the new file's. `raster-gpu/tests/mul_add_hazard.rs` lists the one new `mul_add` site, cold:
the exact products are compared only where the rounded ones already agree, and a subpath's two
defining points are skipped, so an ordinary outline never reaches the call. The six arms against HEAD's six arms say what moved on the
corpus; the round's record has the figures.
