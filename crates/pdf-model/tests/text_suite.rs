//! `pdf-model`'s integration tests of clause 9's fonts and text, linked as one test binary.
//!
//! Its modules: every font type, `CMaps` and `ToUnicode`, the text state and rendering modes, glyph
//! metrics and substitute faces, the font programs a save writes, and the text a reader extracts
//! and selects. Each is a file beside this one, named by a `mod` line below, and a test
//! is `<file>::<test>` — `cargo test -p pdf-model --test text_suite <file>::`, or
//! `cargo nextest run -p pdf-model --test text_suite -E 'test(/^<file>::/)'`.
//!
//! The crate's tests are grouped by subject into nine binaries like this one, and the corpus walks
//! the tier-2 gates name stay binaries of their own; ADR 3086 has the grouping, its measurement and
//! what it costs. `pdf-model` turns Cargo's test discovery off (`autotests = false`), so **a new
//! test file is compiled only once a `mod` line here or in a sibling suite names it**;
//! `tools/conformance/tests/test_targets.rs` fails on a file no suite names.
//!
//! Under `cargo test` the modules share one process (nextest gives each test a process of its own
//! either way), so a module sets no environment variable and changes no working directory.

mod cid_map_stream_spec_audit;
mod composite_font_fixtures;
mod composite_fonts;
mod composite_literal_escape_spec_audit;
mod direct_cid_font_descriptor_spec_audit;
mod encoding_cmap_usecmap_spec_audit;
mod font_cache_identity_audit;
mod font_collections;
mod font_paint_type_audit;
mod font_subset_reader_audit;
mod glyph_clip_direction;
mod logical_order;
mod machine_cff_face_written;
mod machine_face_written;
mod null_font_substitute;
mod ocr_text_layer;
mod opentype_selection_audit;
mod panose;
mod refused_font_metrics;
mod refused_glyphs;
mod selection_geometry;
mod silent_fonts;
mod simple_font_standard_fill_spec_audit;
mod simple_font_widths_audit;
mod subset_names_in_one_save;
mod substitute_unreached_codes;
mod substituted_shapes;
mod text_clip_pixels_spec_audit;
mod text_clipping_audit;
mod text_layer;
mod text_positioning_audit;
mod text_render_modes;
mod text_state;
mod text_state_residual_audit;
mod text_state_units_audit;
mod text_state_vertical_spec_audit;
mod tounicode_bfrange_sequence_destinations_spec_audit;
mod tounicode_keeps_source_code_length_spec_audit;
mod tounicode_usecmap_stream_base;
mod tounicode_usecmap_stream_inheritance_spec_audit;
mod truetype_standard_fill_spec_audit;
mod type1;
mod type3;
mod type3_image_restrictions_spec_audit;
mod type3_residual_audit;
mod type3_shape_state_restoration;
mod type3_spec_audit;
mod unreadable_program_fallback_control_audit;
mod unreadable_program_substitute;
mod unstated_widths;
mod vertical_forms;
mod vertical_program_metrics_audit;
