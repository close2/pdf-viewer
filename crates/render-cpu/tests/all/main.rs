//! `render-cpu`'s integration tests, linked as one test binary.
//!
//! Each file beside this one was a test binary of its own, and is now a module of this one, so
//! a test is named `all::<file>::<test>` and is filtered as `cargo nextest run -p render-cpu
//! --test all -E 'test(/^<file>::/)'` or `cargo test -p render-cpu --test all <file>::`. One
//! binary links the crate's graph once instead of once a file; ADR 3036 has the measurement and
//! what it costs. A new test file joins by being placed in this directory and named in the list
//! below: a file in `tests/` itself would be a binary of its own again.
//!
//! Under `cargo test` the modules share one process (nextest gives each test a process of its own
//! either way). None of them sets the environment, the working directory or a global, and that is
//! the condition a new one keeps.

mod a_level_where_the_paint_is_the_pixel;
mod abutting_rectangles;
mod anisotropic_sub_pixel_stroke;
mod anti_aliased_shading;
mod basic_composite_partial_alpha_spec_audit;
mod clip_bands;
mod clip_intersection;
mod clip_that_cuts_nothing;
mod culled_window;
mod dash_subpath_restart;
mod degenerate_subpath;
mod drawn_ahead_in_strips;
mod edge_coverage;
mod empty_clip;
mod fill_rules;
mod folding_stroke_under_a_clip;
mod four_component_page;
mod glimpsed_draw;
mod group_clip_intersection;
mod group_constructions;
mod headless_render;
mod image_centre_across_a_cut;
mod image_clip_intersection;
mod image_placement;
mod interrupted_draw;
mod knockout_element_coverage;
mod overlapping_portions;
mod page_past_the_tile;
mod path_closure_raster_audit;
mod rectangle_across_a_cut;
mod sampled_shading;
mod separable_blend_oracle_audit;
mod shading_placement;
mod soft_mask;
mod strip_cut_exactness;
mod strip_parallelism;
mod stripe_rows;
mod stroke_cuts;
mod stroke_fill_geometry_spec_audit;
mod stroke_join_shapes_audit;
mod stroke_parameters_spec_audit;
mod stroke_wider_than_its_curve;
mod stroke_width;
mod stroked_shading_background;
mod thin_segment_ladder;
mod transfer_edge;
mod transparent_degenerate_spec_audit;
mod window_on_the_page_grid;
mod zero_area_clip;
mod zero_area_fill;
