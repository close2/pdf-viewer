# 3095 — A stroke paints a mesh through the fill's mesh raster, on every backend

Session 2098. Status: **implemented.** Context: ADR 3074's record (a stroke of an unflagged mesh shading was
refused by `render-cpu` and Vello and drawn by quorra), ADR 0051 (`MeshRaster`). Clauses: ISO 32000-2 §8.7.4.1,
§8.7.4.5.5 to §8.7.4.5.8.

## Why each refused

A stroke's brush comes from a gradient: `render-cpu`'s `mark_brush` asks `ramp_shader` (axial, radial), then
`paint`, whose `shading::shader` has no shader for a mesh and returns `None` — `CpuRasterError::UnsupportedPaint`.
Vello's `stroke_brush` asks `brush_for`, which refuses a mesh by name (`GpuRasterError::UnsupportedPaint`). A
fill never asks either for a mesh: both backends rasterise the mesh with `pdf_render::MeshRaster` and draw it
clipped to the shape. quorra already stroked a mesh with its `Paint::Mesh`. A flagged mesh was drawn by all three
only because ADR 3074's raster door is asked first.

## The construction

§8.7.4.1 lets a shading pattern paint with "S (stroke)" as with "f (fill)", and nothing in §8.7.4.5.5 to
§8.7.4.5.8 is a fill's alone. So the stroke takes the fill's raster:

- **`render-cpu`**: `shading::mesh_shader` builds `MeshRaster::build_within` over the stroke's `Extent` (the ink
  bounds and reach `raster_shader` uses) on the page's grid, placed by the shading's transform as `fill_mesh`
  places it, and returns it as the nearest-neighbour pattern `raster_shader` returns (one helper,
  `device_raster_pattern`, now places both). `mark_brush` asks it for any mesh; a mesh that reaches none of the
  surface is a transparent brush, never a refusal.
- **Vello**: `stroked_as_a_raster` adds a mesh to `is_rastered()`, so `stroke_brush` asks no brush, and
  `stroke_filtered` draws `shading::fill_mesh` inside the layer clipped to the stroke's outline and dots, as
  `encode_fill` draws it inside the shape.

## Tests

`test_scenes::shading_anti_alias_stroked(false)`, whose third stroke is an unflagged parametric mesh:
`render-cpu/tests/all/anti_aliased_shading.rs::an_unfiltered_mesh_stroke_paints_the_fills_pixels_inside_it`,
`render-raster/tests/headless_quorra.rs::cpu_and_quorra_agree_on_an_unfiltered_mesh_stroke`,
`render-gpu/tests/headless_gpu.rs::the_gpu_draws_an_unfiltered_mesh_stroke`. Each of the CPU and Vello tests
fails with its door shut (the backend refuses the paint), and quorra's fails because its oracle refused.
