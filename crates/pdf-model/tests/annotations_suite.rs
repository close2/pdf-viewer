//! `pdf-model`'s integration tests of §12.5's annotations, linked as one test binary.
//!
//! Its modules: every annotation subtype's dictionary and appearance, borders, flags, popups and
//! markup, and XFDF's annotations. Each is a file beside this one, named by a `mod` line below, and
//! a test
//! is `<file>::<test>` — `cargo test -p pdf-model --test annotations_suite <file>::`, or
//! `cargo nextest run -p pdf-model --test annotations_suite -E 'test(/^<file>::/)'`.
//!
//! The crate's tests are grouped by subject into nine binaries like this one, and the corpus walks
//! the tier-2 gates name stay binaries of their own; ADR 3086 has the grouping, its measurement and
//! what it costs. `pdf-model` turns Cargo's test discovery off (`autotests = false`), so **a new
//! test file is compiled only once a `mod` line here or in a sibling suite names it**;
//! `tools/conformance/tests/test_targets.rs` fails on a file no suite names.
//!
//! Under `cargo test` the modules share one process (nextest gives each test a process of its own
//! either way), so a module sets no environment variable and changes no working directory.

mod annotation_appearance_audit;
mod annotation_border_audit;
mod annotation_dictionary_audit;
mod annotation_flags_audit;
mod annotation_oc_flags_audit;
mod annotation_output_flags_spec_audit;
mod annotation_state_model_defaults_spec_audit;
mod annotation_states_audit;
mod annotation_view_matrix_spec_audit;
mod annotations;
mod cloudy_border_audit;
mod file_attachment_icon_audit;
mod free_text_annotation_spec_audit;
mod freetext_cloud_border_spec_audit;
mod ink_annotation_spec_audit;
mod line_caption_spec_audit;
mod line_ending_endpoint_spec_audit;
mod link_default_border_spec_audit;
mod link_highlighting_audit;
mod link_highlighting_residual_audit;
mod links;
mod markup_annotation_audit;
mod markup_stored_appearance_spec_audit;
mod polygon_polyline_spec_audit;
mod popup_page_appearance_audit;
mod print_intent;
mod replacement;
mod square_circle_spec_audit;
mod stored_annotation_blend_spec_audit;
mod tab_order;
mod text_annotation_spec_audit;
mod text_markup_orientation_audit;
mod watermark_defaults_audit;
mod xfdf_annotations;
