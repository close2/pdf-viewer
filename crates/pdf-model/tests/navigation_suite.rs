//! `pdf-model`'s integration tests of the document-level features of clauses 12 and 14, linked as one test binary.
//!
//! Its modules: outlines, destinations, actions, page labels, articles, thumbnails, collections,
//! file specifications and embedded files, metadata, viewer preferences, logical structure, tagged
//! PDF and accessibility, measurement and media. Each is a file beside this one, named by a `mod`
//! line below, and a test
//! is `<file>::<test>` — `cargo test -p pdf-model --test navigation_suite <file>::`, or
//! `cargo nextest run -p pdf-model --test navigation_suite -E 'test(/^<file>::/)'`.
//!
//! The crate's tests are grouped by subject into nine binaries like this one, and the corpus walks
//! the tier-2 gates name stay binaries of their own; ADR 3086 has the grouping, its measurement and
//! what it costs. `pdf-model` turns Cargo's test discovery off (`autotests = false`), so **a new
//! test file is compiled only once a `mod` line here or in a sibling suite names it**;
//! `tools/conformance/tests/test_targets.rs` fails on a file no suite names.
//!
//! Under `cargo test` the modules share one process (nextest gives each test a process of its own
//! either way), so a module sets no environment variable and changes no working directory.

mod accessibility;
mod action_next_occurrences_audit;
mod articles;
mod artifact_reporting_audit;
mod associated_file_examples;
mod associated_file_examples_audit;
mod attachments_filed;
mod collections;
mod content_rectangle;
mod destinations;
mod dpart_af_current_shape_spec_audit;
mod embedded_files_spec_audit;
mod embedded_flate_checksum_spec_audit;
mod empty_actualtext_spec_audit;
mod file_specification_writer_audit;
mod file_specifications_audit;
mod guidelines;
mod guidelines_decision_audit;
mod indirect_file_parameters_spec_audit;
mod logical_structure_example;
mod marked_content_scope;
mod measurement;
mod measurement_12_9_spec_audit;
mod media_screen_background;
mod media_viability;
mod missing_attribute_class_spec_audit;
mod movie_poster;
mod named_pages_spec_audit;
mod named_property_actualtext_spec_audit;
mod navigation_12_3_12_4_12_6_spec_audit;
mod nested_mcaf_spec_audit;
mod outlines;
mod page_label_missing_style_spec_audit;
mod page_labels;
mod reference_page_label_limit_spec_audit;
mod rendition_scripts;
mod structure;
mod structure_14_6_14_9_spec_audit;
mod structure_attribute_stream_audit;
mod structure_limit_reporting_audit;
mod structure_mcr_page_spec_audit;
mod structure_objr_page_spec_audit;
mod tagged_pdf_fixtures;
mod thumbnails;
mod uf_embedded_stream_selection_spec_audit;
mod viewer_preference_defaults_spec_audit;
mod writing_mode_edges_spec_audit;
