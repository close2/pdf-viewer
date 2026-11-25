//! `pdf-model`'s integration tests of clause 8's content and drawing, linked as one test binary.
//!
//! Its modules: content streams and their operators, the graphics state, paths, clipping and scan
//! conversion, shadings and patterns, form `XObjects`, optional content (§8.11) and page geometry.
//! Each is a file beside this one, named by a `mod` line below, and a test
//! is `<file>::<test>` — `cargo test -p pdf-model --test content_suite <file>::`, or
//! `cargo nextest run -p pdf-model --test content_suite -E 'test(/^<file>::/)'`.
//!
//! The crate's tests are grouped by subject into nine binaries like this one, and the corpus walks
//! the tier-2 gates name stay binaries of their own; ADR 3086 has the grouping, its measurement and
//! what it costs. `pdf-model` turns Cargo's test discovery off (`autotests = false`), so **a new
//! test file is compiled only once a `mod` line here or in a sibling suite names it**;
//! `tools/conformance/tests/test_targets.rs` fails on a file no suite names.
//!
//! Under `cargo test` the modules share one process (nextest gives each test a process of its own
//! either way), so a module sets no environment variable and changes no working directory.

mod compatibility_operator_pixels_spec_audit;
mod content_resources_audit;
mod content_streams;
mod content_window;
mod contents_q_stack_cross_part_spec_audit;
mod coordinate_graphics_state_spec_audit;
mod coordinates_8_3_10_4_spec_audit;
mod damaged_content_streams;
mod degenerate_shading_bbox_spec_audit;
mod degenerate_shading_geometry_spec_audit;
mod degenerate_tiling_bbox_clip_spec_audit;
mod extgstate_defaults_audit;
mod extgstate_font_valid_consumer_spec_audit;
mod extgstate_missing_line_keys_audit;
mod form_xobjects_audit;
mod graphics_stack_paths_audit;
mod graphics_state_paths_spec_audit;
mod gwg_output_suite;
mod hostile_geometry;
mod leftover_operands;
mod line_dash;
mod line_parameters;
mod missing_resources;
mod named_page_geometry_spec_audit;
mod nested_content_window;
mod optional_content;
mod optional_content_config_audit;
mod optional_content_nested_spec_audit;
mod page_geometry;
mod page_resource_render_spec_audit;
mod path_construction;
mod path_painting_audit;
mod reference_xobjects;
mod render_real_pdf;
mod rendering_10_5_10_8_spec_audit;
mod repair_classes;
mod resources_7_8_3_spec_audit;
mod scan_conversion_collapsed_by_transform;
mod scan_conversion_shapes_without_area;
mod shading_continuations_spec_audit;
mod shading_parameters_spec_audit;
mod shading_pattern_matrix_spec_audit;
mod shading_pattern_stream_spec_audit;
mod shadings;
mod strip_parallelism;
mod tiling;
mod tiling_patterns_audit;
mod tiling_signed_steps_spec_audit;
mod type4_edge_flags_spec_audit;
mod type5_function_spec_audit;
mod type5_vertex_padding_spec_audit;
mod undefined_painting_ignored;
mod xobjects_oc_8_9_8_11_spec_audit;
