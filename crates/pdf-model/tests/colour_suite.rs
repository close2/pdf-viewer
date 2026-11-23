//! `pdf-model`'s integration tests of §8.6's colour and clause 10's rendering, linked as one test binary.
//!
//! Its modules: colour spaces, ICC profiles, Indexed, Separation and `DeviceN`, spot colourants,
//! overprint, rendering intents, transfer functions and black generation, and output intents. Each
//! is a file beside this one, named by a `mod` line below, and a test
//! is `<file>::<test>` — `cargo test -p pdf-model --test colour_suite <file>::`, or
//! `cargo nextest run -p pdf-model --test colour_suite -E 'test(/^<file>::/)'`.
//!
//! The crate's tests are grouped by subject into nine binaries like this one, and the corpus walks
//! the tier-2 gates name stay binaries of their own; ADR 3086 has the grouping, its measurement and
//! what it costs. `pdf-model` turns Cargo's test discovery off (`autotests = false`), so **a new
//! test file is compiled only once a `mod` line here or in a sibling suite names it**;
//! `tools/conformance/tests/test_targets.rs` fails on a file no suite names.
//!
//! Under `cargo test` the modules share one process (nextest gives each test a process of its own
//! either way), so a module sets no environment variable and changes no working directory.

mod black_generation_paint;
mod black_generation_transfer_audit;
mod calrgb_literal_samples_spec_audit;
mod cmyk_image_overprint_spec_audit;
mod colour_8_6_8_7_spec_audit;
mod colour_paths;
mod default_colour_spaces_audit;
mod device_rgb_reset_spec_audit;
mod devicen_decode_defaults_spec_audit;
mod extgstate_overprint_precedence;
mod gray_display_intents_spec_audit;
mod halftone_transfer_public_spec_audit;
mod icc_alternate_range_spec_audit;
mod icc_paint_context;
mod indexed_lab_lookup_spec_audit;
mod indexed_out_of_range;
mod indexed_rounding_initial_spec_audit;
mod mixed_paint_context;
mod multitone_examples;
mod output_intent_page_scope_spec_audit;
mod overprint;
mod overprint_group_row_spec_audit;
mod pattern_space_and_steps;
mod pattern_tint_context;
mod pdfa_grey;
mod rendering_intent;
mod special_space_initial_tints_spec_audit;
mod spot_colourants;
mod spot_planes;
mod spot_press;
mod transfer_functions;
