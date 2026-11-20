# 3333 — The thinnest line is stroked in device space, and has no mitre

Session 2158. Status: accepted. Context: audit 2155's findings B01 and B03
(`doc/reviews/2155-specification-audit-8-4-8-5.md`), ADRs 0028, 0480, 0697, 0702, 2530; raster's
`raster/doc/adr/0085`.

## The clauses

ISO 32000-2 §8.4.3.2: "A line width of 0 shall denote the thinnest line that can be rendered at device
resolution: 1 device pixel wide." §8.4.3.5: "When the line width is zero, the miter length is zero."
§10.7.5's NOTE makes its promoted stroke "equivalent to the effect produced by setting the line width
to 0", so both answers below hold of a stroke `/SA` promotes as well.

## What was wrong

`Stroke::device_width` substitutes one pixel divided by the transform's largest stretch, a width in the
path's own space. Under a similarity that is one pixel in every direction; under `4 0 0 1` it is one
pixel across the stretched axis and a quarter of one across the other. The comment answered that with
§8.4.3.2's orientation sentence, which is about a width stated in user space — and a zero width is
stated at device resolution, so it has no orientation to vary with. And the substituted pixel was
handed to each stroker with the state's join, so a 14° corner grew a two-pixel spike that §8.4.3.5
gives a length of zero.

## Decision

1. **One predicate, in `pdf-render`**: `Stroke::is_thinnest_line(to_device)` — a width of zero, or one
   §10.7.5 promotes. `device_width` substitutes for exactly these.
2. **No mitre.** `Stroke::as_drawn(to_device)` is the stroke a backend draws: unchanged, except that a
   mitred join of the thinnest line becomes a bevel, since a mitre of length zero reaches no further.
   `render-cpu` and `render-gpu` stroke `as_drawn`; raster resolves its width per placement (its ADR
   0085), so its encoder asks the same question there (`raster_gpu::raster::as_drawn`).
3. **Device space where no path-space width will do.** `Stroke::strokes_in_device_space(to_device)`
   holds of the thinnest line under a transform whose two stretches differ by more than `grid_fit`'s
   `ALIKE` (1/1024). Such a stroke is drawn one pixel wide on the device's own grid, dashed first in
   the path's space where §8.4.3.6's lengths are:
   - `render-cpu`: `tiny-skia`'s hairline (width 0 under the placement), which dashes in path space,
     strokes on the device grid and has no joins; `convert.rs` already held it equal to a one-pixel
     stroke at every similarity (`tests/all/stroke_width.rs`).
   - `render-gpu`: the dashed shape mapped by the transform's linear part
     (`pdf_render::device_hairline` states the same restatement for display-list paths), stroked one
     unit wide under the translation alone, the brush carried across the same linear map (trap 2).
   - `render-raster`: raster's own expansion, which is in device space with the width it resolves per
     placement; the path-space expansion ADR 0702 chose for unequal scales is no longer taken for it.
   `pdf_render::thin_segment` keeps such a stroke out of its population, whose pen is the ellipse a
   path-space width maps to.

## Cost and what is left

Three constructions of one shape, one per library, each tested against the clause on every backend
(`render-raster/tests/thinnest_line_on_every_backend.rs`). Left: §8.5.3.2's dot of a zero-width line
(a degenerate subpath under a round cap) is still a disc of the path-space width, so under unequal
scales it is an ellipse one pixel across its longer axis.
