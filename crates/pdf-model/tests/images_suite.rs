//! `pdf-model`'s integration tests of §8.9's images and their filters, linked as one test binary.
//!
//! Its modules: sampled and inline images, image masks, decode arrays, bit depths and predictors,
//! and the DCT, JPX, JBIG2 and CCITT decoders. Each is a file beside this one, named by a `mod`
//! line below, and a test
//! is `<file>::<test>` — `cargo test -p pdf-model --test images_suite <file>::`, or
//! `cargo nextest run -p pdf-model --test images_suite -E 'test(/^<file>::/)'`.
//!
//! The crate's tests are grouped by subject into nine binaries like this one, and the corpus walks
//! the tier-2 gates name stay binaries of their own; ADR 3086 has the grouping, its measurement and
//! what it costs. `pdf-model` turns Cargo's test discovery off (`autotests = false`), so **a new
//! test file is compiled only once a `mod` line here or in a sibling suite names it**;
//! `tools/conformance/tests/test_targets.rs` fails on a file no suite names.
//!
//! Under `cargo test` the modules share one process (nextest gives each test a process of its own
//! either way), so a module sets no environment variable and changes no working directory.

mod alternate_images_audit;
mod banded_decodes;
mod bit_depths;
mod ccitt;
mod ccitt_bound;
mod ccitt_eob_rows_spec_audit;
mod colour_transform;
mod dct_components;
mod decode_ahead;
mod decode_arrays;
mod decode_element_types_audit;
mod image_coordinates_audit;
mod image_dictionary_consistency_audit;
mod image_dimensions;
mod image_extent;
mod image_interpolation_audit;
mod image_masking_audit;
mod image_masks;
mod image_reuse;
mod image_samples_decode_audit;
mod inline_decode_public_spec_audit;
mod inline_filter_chain_spec_audit;
mod inline_image_abbreviations;
mod inline_images;
mod inline_images_dictionary_audit;
mod inline_images_residual_audit;
mod jbig2;
mod jbig2_filter_chain_audit;
mod jpx_channels;
mod jpx_decode_array;
mod jpx_enumerated_spaces;
mod jpx_mask_input_audit;
mod jpx_ordinary_opacity_audit;
mod predictor_image_bpc_spec_audit;
