# 2068 — The pin moves to Rust 1.100

Session 2068, a toolchain round in the audit worktree. No ledger row; ADR 2950. The slice is the bump
ADR 2910 held. **The premise** held in part: 1.100.0 is installed, but step 2 was already built, as round
2049's `pdf_sandbox::beside` (ADR 2914), and under 1.100 every test found its program with no worker
named. ADR 2950 states the bump and cites ADR 2914. Main moved to `bf59984d` (the second audit) during
the round; the work was rebased onto it, and every gate below is on the rebased tree.

**The bump (ADR 2950).** The pin `1.100.0`, `rust-version` and `msrv` `1.100`. Clippy 1.100 named 1056
sites at `ce6bc1d8`: `chunks_exact_to_as_chunks` 410, `assert_is_empty` 294, `map_unwrap_or` 254,
`manual_midpoint` 45, unfulfilled expectations 38, `manual_is_variant_and` 8, `manual_bit_width` 4, and
one each of `needless_nonzero_get`, `needless_range_loop`, `useless_format`; the second audit brought 39
more and 8 expectations. Cargo's lints removed six unused workspace entries and moved `pdf-model`'s
`pdf-spec` to a dev-dependency. Round 2047's patch applied with one conflict (`launch_path.rs`, kept
main's `beside::program`).

**The owner's audit tests, lint form only (A875).** 51 files, 28 of the first audit and 23 of the second;
every assertion's operands, inputs and expected outcome unchanged (`.rs` omitted):
`crates/pdf-colour/tests/`: `indirect_function_colour_spec_audit`. `crates/pdf-model/examples/`: `absence_audit`.
`crates/pdf-model/tests/`: `annotation_border_audit`, `annotation_states_audit`, `annotation_view_matrix_spec_audit`, `cloudy_border_audit`, `content_resources_audit`, `degenerate_shading_bbox_spec_audit`, `degenerate_shading_geometry_spec_audit`, `extgstate_missing_line_keys_audit`, `fdf_field_dictionary_audit`, `fdf_template_file_spec_audit`, `field_aa_noninheritance_spec_audit`, `file_attachment_icon_audit`, `font_paint_type_audit`, `freetext_cloud_border_spec_audit`, `inherited_resources_spec_audit`, `jbig2_filter_chain_audit`, `signature_field_lock_spec_audit`, `signature_fields_spec_audit`, `structure_mcr_page_spec_audit`, `structure_objr_page_spec_audit`, `text_annotation_spec_audit`, `text_state_residual_audit`, `text_strings_spec_audit`, `trees_rectangles_audit`, `type5_vertex_padding_spec_audit`, `unreadable_program_fallback_control_audit`, `unreadable_program_substitute`, `variable_text_resource_merge_audit`, `variable_text_tx_splice_audit`, `vertical_program_metrics_audit`, `widget_annotation_spec_audit`, `writing_mode_edges_spec_audit`.
`crates/pdf-syntax/tests/`: `indirect_filter_parameters_spec_audit`. `crates/pdf-transform/tests/`: `redaction_projection_spec_audit`. `crates/render-cpu/tests/`: `path_closure_raster_audit`. `crates/render-raster/tests/`: `degenerate_shading_bbox_spec_audit`, `degenerate_shading_geometry_spec_audit`.
`crates/viewer-core/tests/`: `agl_sequences_spec_audit`, `annotation_print_appearance_audit`, `document_trigger_timing_audit`, `enforced_preferences`, `file_attachment_annotation_audit`, `rendition_preconditions_spec_audit`, `sound_movie_annotation_audit`, `unknown_annotation_pointer_spec_audit`.
`crates/viewer-gtk/src/host/printing/`: `print_defaults_settings_spec_audit`. `crates/viewer-host/tests/`: `required_empty_field_report_and_policy`, `watermark_nup_consumer_audit`. `crates/viewer-ui/tests/`: `outline_colour_spec_audit`.

**The bands.** The counted half put WTPDF's and EC3's opens and `bug1815476`'s first frame outside;
four children each, read by function against round 2065's 1.97.1 profiles, place every move in
dependencies' loops and unchanged functions (ADR 2950). Restated by the file's rule: WTPDF open 9463 ..
9656 to 9997 .. 10200, EC3 open 52862 .. 53931 to 55063 .. 56176, EC3 frame 156099 .. 159254 to 157469 ..
160652, `bug1815476` frame 64104 .. 65400 to 62847 .. 64118, `xfa_filled_imm1344e` frame 3204 .. 3269 to
3234 .. 3300. Seven others moved under 0.5% and keep theirs. No turn band moved.

**Gates.** Under 1.100.0, rebased: fmt 0; clippy `-D warnings --workspace --all-targets` 0; fuzz
`cargo check --bins` 0; rustdoc `-D warnings --document-private-items` 0; nextest under `xvfb-run -a` 100
(10410 run, 121 failed: all main's own, the second audit's expected failures and five conformance gates).
`launch_path` with clocks 101 twice, all 36 counted figures inside: run 1 (load 28 to 245) `opt_demo` cold
open 1.131 against 1.000, run 2 (load 245 to 38) WTPDF cold open 5.625 against 5.490; none outside in
both. `turn_path` 0 (33 banded, 18 judged, 0 outside). Conformance after `git add -N` 101, main's five only.

**Left.** `cargo deny` fails on RUSTSEC-2026-0543 (`chrono` 0.4.45 unmaintained), an advisory and not
the bump: the lock file is unchanged. `proc-macro-error2` 2.0.1 carries a future-incompatibility note.
