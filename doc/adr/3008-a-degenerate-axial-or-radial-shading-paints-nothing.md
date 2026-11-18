# 3008 — A coincident axial axis and two zero radial radii paint nothing, decided once

Session 2080. Status: **implemented.** Context: review 2038 (findings B70, B71, T126, T128). Clauses: ISO 32000-2
§8.7.4.5.3 Table 79, §8.7.4.5.4 Table 80.

Table 79: "If the starting and ending coordinates are coincident (x 0 =x 1 and y 0 =y 1 ) nothing shall be painted."
Table 80: "If one radius is 0 , the corresponding circle shall be treated as a point; if both are 0 , nothing shall be
painted." `render-cpu` handed both to `tiny-skia`, which painted a solid colour over the whole region for the axis
and refused the radial outright; `render-raster` already painted nothing.

`pdf_render::Shading::paints_nothing` states the two conditions once (trap 2), and `render-cpu` asks it before a
shaded fill (`filled_as_a_raster`) and a shaded stroke. `/Background` is not the shading's geometry: Table 77 has a
direct `sh` ignore it, and a pattern fill that states one keeps the background path it had.

Tests: `pdf-model::degenerate_shading_geometry_spec_audit` (both), `render-raster::degenerate_shading_geometry_spec_audit`.
