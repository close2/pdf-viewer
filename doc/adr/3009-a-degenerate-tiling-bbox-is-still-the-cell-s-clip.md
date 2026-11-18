# 3009 — A tiling pattern's zero-height or zero-width `/BBox` is still the cell's clip

Session 2080. Status: **implemented.** Supersedes ADR 0056 on this point (a degenerate box left the cell unclipped).
Context: review 2038 (finding B20). Clauses: ISO 32000-2 §8.7.3.1 Table 74, §10.7.4.

Table 74: "These boundaries shall be used to clip the pattern cell", and its NOTE 1: "A BBox of zero height or width
will still paint one pixel." The reading kept in `pdf_model::content::pattern::tiling` dropped such a box, because a
zero-area clip once removed everything on both backends; the cell then painted marks far outside the box. ADRs 1060
and 1064 have since made every backend build a clip's region by §10.7.4's fill rule — "A zero-width or zero-height
rectangle paints a line 1 pixel wide" — through `pdf_render::clip_region`, which is exactly NOTE 1's one pixel.

Now any box of four finite numbers is normalised and kept as the cell's clip; only a box that is not one (missing,
short, or non-finite) leaves the cell unclipped, as before.

Test: `pdf-model::degenerate_tiling_bbox_clip_spec_audit`.
