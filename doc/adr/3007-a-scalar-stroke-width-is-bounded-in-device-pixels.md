# 3007 — A scalar device stroke width is bounded in device pixels as well as by ratio

Session 2080. Status: **implemented.** Amends ADR 0702's route choice in `render-raster::stroke`. Context: review
2038 (findings B59, T111). Clause: ISO 32000-2 §8.4.3.2.

§8.4.3.2: "If the CTM specifies scaling by different factors in the horizontal and vertical dimensions, the thickness
of stroked lines in device space shall vary according to their orientation. The actual line width achieved can
differ from the requested width by as much as 2 device pixels, depending on the positions of lines with respect to
the pixel grid." `render-raster` hands the scene one scalar width — the transform's larger stretch — wherever the
ratio of the singular values is under 1.01, and expands the stroke in path space otherwise. A ratio does not bound
the error: a 1000-unit line under `scale(1.009, 1)` was 1008 pixels across the axis the scale leaves alone. The two
pixels the clause allows belong to grid placement, not to a width read off the wrong direction.

Now the scalar route also needs `path_width · (max − min) ≤ 0.25` device pixels (`MAX_SCALAR_WIDTH_ERROR`). The
clause states no tolerance for this error, so a quarter pixel is a choice: under what an anti-aliased edge shows,
and it keeps every stroke under 25 pixels wide on the scalar route at a 1% ratio. Because the choice now reads the
view's magnification, a scene that took the scalar route under a transform that is not a similarity is marked as
having read the view (ADR 0702), so a later magnification re-encodes it.

Tests: `render-raster::near_similarity_stroke_spec_audit` (both), `anisotropic_stroke_width_spec_audit`.
