//! `pdf-model`'s integration tests of clause 7's syntax and file structure, linked as one test binary.
//!
//! Its modules: objects, strings, names, dates, name and number trees, the page tree and inherited
//! page attributes, filters' faults, damaged and linearized files, budgets against hostile input,
//! encryption's permissions, and §7.5.6's incremental update as `saving` writes it. Each is a file
//! beside this one, named by a `mod` line below, and a test
//! is `<file>::<test>` — `cargo test -p pdf-model --test syntax_suite <file>::`, or
//! `cargo nextest run -p pdf-model --test syntax_suite -E 'test(/^<file>::/)'`.
//!
//! The crate's tests are grouped by subject into nine binaries like this one, and the corpus walks
//! the tier-2 gates name stay binaries of their own; ADR 3086 has the grouping, its measurement and
//! what it costs. `pdf-model` turns Cargo's test discovery off (`autotests = false`), so **a new
//! test file is compiled only once a `mod` line here or in a sibling suite names it**;
//! `tools/conformance/tests/test_targets.rs` fails on a file no suite names.
//!
//! Under `cargo test` the modules share one process (nextest gives each test a process of its own
//! either way), so a module sets no environment variable and changes no working directory.

mod contents_entry;
mod damaged_char_procs;
mod damaged_page_dictionaries;
mod deep_name_tree_spec_audit;
mod deep_number_tree_spec_audit;
mod deep_page_tree;
mod deep_page_tree_order_spec_audit;
mod direct_info_spec_audit;
mod function_faults;
mod hostile_budgets;
mod hostile_functions;
mod indirect_annotation_values_spec_audit;
mod indirect_differences_spec_audit;
mod indirect_extgstate_values_spec_audit;
mod indirect_link_action_spec_audit;
mod indirect_shading_values_spec_audit;
mod inherited_resources_spec_audit;
mod linearized_cut_chain;
mod named_appearances;
mod names_are_bytes;
mod numbers_without_digits;
mod page_tree_nodes;
mod public_key_permissions_spec_audit;
mod restrictions;
mod sampled_function_extent;
mod saving;
mod shared_resources;
mod stream_faults_by_name;
mod text_strings_spec_audit;
mod trees_rectangles_audit;
