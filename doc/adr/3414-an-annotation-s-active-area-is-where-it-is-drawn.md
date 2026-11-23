# 3414 — An annotation's active area is where §12.5.3 draws it

Session 2178. Status: accepted. Context: audit 2175 (B01), ADRs 0168, 0172, 2813.

## Decision

A `NoZoom` or `NoRotate` annotation is drawn moved about its `/Rect`'s upper-left corner, and
§12.5.5's rollover and down appearances and §12.6.3's triggers belong to "the annotation's active
area". Every hit test (`view::annotations::reaches`) maps the point through the **inverse** of the
drawing's own `ViewGeometry::adjustment`, with `view_flags`'s subtype readings (a `Text` note is
always both; text markup never), and then tests `/Rect` or a link's `/QuadPoints` unaltered. One
transform for both, so the drawing and the pointer cannot disagree. The magnification is the view
state's; a caller with none (`field_at`) has none, and `NoZoom` then moves nothing, as it does for
the drawing (ADR 2813).
