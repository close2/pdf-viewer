//! `pdf-model`'s integration tests of §12.7's interactive forms and §12.8's signatures, linked as one test binary.
//!
//! Its modules: fields and widgets, variable text, FDF and XFA, signatures and their locks, field
//! scripts, submission and form actions. Each is a file beside this one, named by a `mod` line
//! below, and a test
//! is `<file>::<test>` — `cargo test -p pdf-model --test forms_suite <file>::`, or
//! `cargo nextest run -p pdf-model --test forms_suite -E 'test(/^<file>::/)'`.
//!
//! The crate's tests are grouped by subject into nine binaries like this one, and the corpus walks
//! the tier-2 gates name stay binaries of their own; ADR 3086 has the grouping, its measurement and
//! what it costs. `pdf-model` turns Cargo's test discovery off (`autotests = false`), so **a new
//! test file is compiled only once a `mod` line here or in a sibling suite names it**;
//! `tools/conformance/tests/test_targets.rs` fails on a file no suite names.
//!
//! Under `cargo test` the modules share one process (nextest gives each test a process of its own
//! either way), so a module sets no environment variable and changes no working directory.

mod aform;
mod appearance_state_precedence_spec_audit;
mod canonical_dates;
mod choice_locality_spec_audit;
mod default_appearance_syntax_7_3_spec_audit;
mod delegated_widgets;
mod empty_fields_array_presence_spec_audit;
mod fdf_field_dictionary_audit;
mod fdf_template_count_audit;
mod fdf_template_fields_audit;
mod fdf_template_file_spec_audit;
mod fdf_template_pixels_spec_audit;
mod fdf_utf8_import_spec_audit;
mod field_aa_noninheritance_spec_audit;
mod field_names_buttons_spec_audit;
mod form_actions_spec_audit;
mod form_font_once;
mod form_limits_and_commits;
mod forms_data;
mod forms_values_read_back;
mod forms_values_spec_audit;
mod inherited_reset_defaults_audit;
mod javascript_text_stream_spec_audit;
mod live_field_flags_spec_audit;
mod max_len_live_property_spec_audit;
mod max_len_model;
mod need_appearances_all_visible_widgets_spec_audit;
mod quadding_field_tree_precedence_spec_audit;
mod radios_in_unison;
mod rich_text;
mod rich_text_default_style_noninheritance_spec_audit;
mod script_annotations;
mod script_properties;
mod script_unreachable;
mod script_unread;
mod sequence_budget;
mod signature_field_lock_spec_audit;
mod signature_fields_spec_audit;
mod signed_field_lock_shapes;
mod submission;
mod text_choice_fields_spec_audit;
mod tm_coordinate_name_spec_audit;
mod tm_html_export_spec_audit;
mod usage_rights_annotation_save_spec_audit;
mod value_statements;
mod variable_text;
mod variable_text_da_tm_audit;
mod variable_text_resource_merge_audit;
mod variable_text_resource_metric_audit;
mod variable_text_tx_splice_audit;
mod widget_annotation_spec_audit;
mod xfa_datasets;
mod xfa_resource_forms_audit;
mod xfa_saved_values_audit;
