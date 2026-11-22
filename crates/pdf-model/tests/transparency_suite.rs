//! `pdf-model`'s integration tests of clause 11's transparency, linked as one test binary.
//!
//! Its modules: soft masks, transparency groups, blend modes and compositing, matte and knockout.
//! Each is a file beside this one, named by a `mod` line below, and a test
//! is `<file>::<test>` — `cargo test -p pdf-model --test transparency_suite <file>::`, or
//! `cargo nextest run -p pdf-model --test transparency_suite -E 'test(/^<file>::/)'`.
//!
//! The crate's tests are grouped by subject into nine binaries like this one, and the corpus walks
//! the tier-2 gates name stay binaries of their own; ADR 3086 has the grouping, its measurement and
//! what it costs. `pdf-model` turns Cargo's test discovery off (`autotests = false`), so **a new
//! test file is compiled only once a `mod` line here or in a sibling suite names it**;
//! `tools/conformance/tests/test_targets.rs` fails on a file no suite names.
//!
//! Under `cargo test` the modules share one process (nextest gives each test a process of its own
//! either way), so a module sets no environment variable and changes no working directory.

mod blend_gray_consumer_audit;
mod blended_cmyk_mask;
mod compositing_formula_spec_audit;
mod compositing_notation;
mod damaged_mask_consumer_audit;
mod matte_after_decode_spec_audit;
mod matte_reduced;
mod matte_routes;
mod nonisolated_removal_audit;
mod pattern_shape_opacity_audit;
mod soft_mask_derivation_audit;
mod soft_mask_dictionary_audit;
mod soft_mask_spot_audit;
mod soft_mask_transfer_limits_audit;
mod soft_masks;
mod stroke_composition_pdf_spec_audit;
mod transparency;
mod transparency_clause11_spec_audit;
mod transparency_groups;
mod transparency_repeated_group_spec_audit;
