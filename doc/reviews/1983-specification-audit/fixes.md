# What round 2000 did with each finding of review 1983

Review 1983 left two populations of findings. The first is the 72 failing tests listed in
`expected-failures.json`; section 1 below groups them by shared cause and names the fix for each. The second is
the 112 prose findings, numbered 1 to 112 in section 3 (the numbering of the round's prose-findings list), each
with its disposition. ADRs 2811 to 2820 record the decisions the round took, `doc/questions/Q834` holds the four
questions only the owner can answer, and `expected-failures-after-2000.json` lists what still fails.

## 1. Shared causes

- ISO 32000-2 §7.4.3 (EC3 issue 293, "~>" is a two-character EOD marker). Both ASCII85 routes (`ascii85` and the
  windowed `Ascii85` pump) refuse a `~` that the next byte does not complete; a window ending on `~` waits for its
  next byte. ADR 2811. Tests:
  - pdf-syntax::ascii85_runlength_boundaries ascii85_tilde_followed_by_anything_but_greater_than_is_an_error
  - pdf-syntax::ascii85_runlength_boundaries ascii85_windowed_reader_reports_a_malformed_eod_marker
- §7.5.6, the added trailer carries every entry but Prev. `carry_forward` now copies every merged trailer entry
  except the section's own keys. Test:
  - pdf-syntax::file_structure_audit an_incremental_update_repeats_a_second_class_trailer_entry
- §7.3.10 and §7.5.7, an object is its number and generation, and a reference to another generation is null. The
  object cache holds each number's defined generation; `Document::generation_of` and `defined_id` serve walks
  over object numbers. Tests:
  - pdf-syntax::spec_basic_objects a_cold_wrong_generation_reference_is_undefined
  - pdf-syntax::spec_basic_objects a_warm_wrong_generation_reference_is_undefined
- Annex B.3 / §7.10.5.1 with PLRM3 `bitshift`: right shift is logical (zero fill). ADR 2811, superseding ADR
  0369/0371 on that point. Test:
  - pdf-colour::calculator_operators_spec_audit type4_right_bitshift_inserts_zero_bits_for_negative_values
- §7.10.4, the stitching selector gives x at a degenerate first bound to the first function (Encode 0). ADR 2811.
  Test:
  - pdf-colour::functions_spec_audit stitching_domain_start_bound_uses_its_explicit_degenerate_encode_value
- §7.11.3 Table 43 /Desc in EmbeddedFiles: document-home filing gets a /Desc (`filing::tree_description`). ADR
  2812 section 1, superseding ADR 0814 for tree specs. Test:
  - pdf-model::file_specification_writer_audit document_home_file_spec_has_table43_desc
- §7.9.2.3 Table 246 FDF /Status read through `pdf_syntax::pdf_doc_encoded_string`, so a PDFDocEncoded status is
  not taken for a Unicode marker. Test:
  - pdf-model::text_strings_spec_audit a_pdfdocencoded_fdf_status_is_not_mistaken_for_a_unicode_marker
- §7.9.2.2.2 language escapes inside text strings override catalog and property-list language
  (`pdf_syntax::text_string_languages`, nested Described spans). ADR 2812 section 2. Tests:
  - pdf-model::text_strings_spec_audit a_text_string_language_escape_overrides_property_list_language
  - pdf-model::text_strings_spec_audit a_text_string_language_escape_overrides_the_catalog_language
- Table 111 `d1`: colour-related and transparency ExtGState entries are ignored in an uncoloured glyph
  (`shape_only_glyph`). ADR 2812 section 3 (the issue19634.pdf oracle page moves). Test:
  - pdf-model::type3_residual_audit d1_ignores_color_related_extgstate_changes_and_continues
- §8.11.3.1, a hidden `gs`/`scn` lookup that would change visible state is reported
  (`note_missing_state_resource`). ADR 2812 section 4, qualifying trap 11. Test:
  - pdf-model::content_resources_audit hidden_gs_lookup_that_changes_visible_state_is_reported
- §8.9.5.1 Table 87, an inconsistent image dictionary is an error: Decode length against the colour space, an
  image mask whose BPC is not 1. ADR 2812 section 5. Tests:
  - pdf-model::image_dictionary_consistency_audit an_image_decode_array_with_fewer_pairs_than_its_colour_space_errors
  - pdf-model::image_dictionary_consistency_audit an_image_decode_array_with_extra_pairs_for_its_colour_space_errors
  - pdf-model::image_masking_audit an_image_mask_with_more_than_one_bit_per_component_reports_inconsistency
- §8.9.6.2 (EC3 #215), /Mask and a colour space on an image mask are ignored (`image/mask.rs::stencil`). Test:
  - pdf-model::image_masking_audit ec3_image_mask_ignores_its_own_mask_and_colour_space_entries
- §9.9.1 first bullet, OpenType CFF under a simple Type 1 font uses the encoding's glyph names and the CFF charset
  (`type1_cff_in_opentype`). ADR 2817 section 1. Test:
  - pdf-model::opentype_selection_audit opentype_cff_under_simple_type1_uses_encoding_glyph_names_and_cff_charset
- §12.7.7, a named-page template with an absent or non-name /Type is reported as not a template. Tests:
  - pdf-model::named_pages_spec_audit a_template_without_type_is_reported_as_not_a_template
  - pdf-model::named_pages_spec_audit a_template_with_non_name_type_is_reported_as_not_a_template
- §11.5.3 and §11.4.6, a luminosity soft mask honours a knockout mask group (`knock_out_mask_group`). ADR 2817
  section 2, qualifying ADR 2508. Test:
  - pdf-model::soft_mask_dictionary_audit a_luminosity_soft_mask_respects_a_knockout_group_against_normal_control
- Table 176 and Table 191, /H defaults to I whatever /AP holds (`highlight` lost its `has_down` reading). ADR
  2813, superseding ADR 0123's default. Tests:
  - pdf-model::link_highlighting_residual_audit an_unstated_link_mode_still_defaults_to_invert_when_a_down_appearance_exists
  - pdf-model::widget_annotation_spec_audit omitted_widget_h_retains_the_table_191_invert_default_even_with_a_down_stream
- §12.5.5 and Table 170, printing uses the Normal appearance (`appearance_for` under Purpose::Print). ADR 2813.
  Tests:
  - pdf-model::annotation_appearance_audit printing_uses_normal_even_if_the_view_state_carries_a_pressed_pointer
  - viewer-core::annotation_print_appearance_audit public_print_query_uses_normal_after_pointer_press_selected_down
- §12.5.3 Table 167, NoZoom does not magnify under print. ADR 2813, qualifying ADR 0168/1179/1180. Test:
  - viewer-core::annotation_print_geometry_audit a_screen_nozoom_flag_does_not_shrink_the_printed_annotation_at_view_zoom
- §12.5.6.2 (EC3 #90), written /Contents separates paragraphs with CR (`contents_string`). ADR 2818 section 6.
  Test:
  - pdf-model::markup_annotation_audit writer_uses_cr_for_contents_paragraphs
- Table 178 /LL and /LE, the line proper is L minus the leader vector and endings are drawn at /L. ADR 2818,
  superseding points of ADR 0075 and 0192. Tests:
  - pdf-model::line_ending_endpoint_spec_audit line_endings_use_the_l_coordinate_pairs_even_with_leader_lines
  - pdf-model::line_ending_endpoint_spec_audit positive_ll_places_the_proper_line_opposite_the_clockwise_leaders
- Table 181, a polyline /IC fills closed endings, a polygon /Path is closed, the first ending sits at the first
  vertex. ADR 2818, superseding a point of ADR 1784. Tests:
  - pdf-model::polygon_polyline_spec_audit polyline_ic_fills_closed_endings_but_not_open_endings
  - pdf-model::polygon_polyline_spec_audit polygon_path_strokes_its_closing_edge_while_polyline_path_stays_open
  - pdf-model::polygon_polyline_spec_audit polyline_first_ending_is_at_first_vertex_not_second_vertex
- Table 182, text-markup quads keep four corners and the bottom edge is chosen in the text's frame. ADR 2818,
  superseding a point of ADR 0043. Tests:
  - pdf-model::text_markup_orientation_audit underline_tracks_the_counterclockwise_quad_text_edge_not_its_opposite
  - pdf-model::text_markup_orientation_audit highlight_preserves_all_four_corners_of_a_counterclockwise_trapezoid
- §12.7.4.3, a new widget appearance is built with its /Rect at the origin and BBox [0 0 w h] (`in_form_box`).
  ADR 2818. Test:
  - pdf-model::variable_text_resource_merge_audit a_new_variable_text_appearance_uses_origin_and_widget_dimensions
- §12.7.4.3, quadding is laid out in the font the stored stream's resources name, merged over /DR. ADR 2818.
  Test:
  - pdf-model::variable_text_resource_metric_audit saved_quadding_uses_the_font_that_the_appearance_resource_name_resolves_to
- §9.9.2, sibling subsets in one writer get distinct subset tags (`for_a_file` and `retag`). ADR 2818. Test:
  - pdf-model variable_text::subset_collision_audit::appearance_writer_allocates_distinct_tags_for_sibling_subsets
- §12.3.2.2 Table 149, FitBH top and FitBV left go through the page transform; null XYZ coordinates keep the
  user-space value across zoom change and page turn. ADR 2814, superseding a point of ADR 0162. Tests:
  - viewer-core::destinations_residual_audit fitbh_fits_the_content_width_and_places_top_in_default_user_space
  - viewer-core::destinations_residual_audit fitbv_fits_the_content_height_and_transforms_left_from_user_space
  - viewer-core::destinations_residual_audit xyz_null_coordinates_and_zero_zoom_retain_the_current_window_values
  - viewer-core::destinations_residual_audit xyz_null_coordinates_retain_user_position_when_zoom_changes_on_the_same_page
- §12.4.2 Table 161, an empty page label is a label (`answers.rs::label`; the host caption shows the index
  alone). ADR 2814. Test:
  - viewer-host::page_labels_audit an_explicit_empty_page_label_is_not_replaced_by_the_unlabelled_fallback
- Table 196 and §12.6.2 NOTE 1, a /Next cycle is the action on its own path and repeated occurrences are
  performed. ADR 2814. Test:
  - pdf-model::action_next_occurrences_audit next_array_reads_each_occurrence_even_when_two_entries_share_an_object
- Table 176 QuadPoints, press and hover routing use the link's active region. ADR 2814 (ADR 0177). Test:
  - viewer-core::link_quadpoints_audit link_pressed_highlight_uses_quadpoints_active_region_not_the_larger_rect
- Table 29, the catalog OpenAction is performed at first present. ADR 2814. Test:
  - viewer-core::action_triggers_spec_audit catalog_open_action_executes_a_supported_non_goto_action
- Table 197 PV and PI follow the visibility of pages in the arrangement. ADR 2814, superseding a point of ADR
  0164. Tests:
  - viewer-core::page_visibility_triggers_audit newly_visible_neighbor_gets_pv_before_it_becomes_current
  - viewer-core::page_visibility_triggers_audit page_remaining_visible_does_not_get_pi_when_current_page_changes
- Table 196, scripts and non-script actions run in one order along a link's chain. ADR 2814 section 4
  (ADR 1602). Test:
  - viewer-core::mixed_action_order_audit a_next_script_observes_preceding_non_script_action_state
- §12.5.6.12, a "when opened" subtype gets its own window without a /Popup. ADR 2814 (ADR 1622/1723). Test:
  - viewer-core::stamp_popup_spec_audit opening_a_stamp_without_a_popup_reference_still_displays_its_associated_note
- §12.5.6.15, a present empty /Contents wins over /Desc. ADR 2814 (ADR 0295). Test:
  - viewer-core::file_attachment_annotation_audit a_present_empty_attachment_contents_does_not_reveal_the_filespec_description
- Table 225 and §12.8.5.2, `signatures()` walks whatever /SigFlags says. ADR 2814 (ADR 0181). Tests:
  - viewer-core::sigflags_discovery_audit absent_sigflags_does_not_hide_a_candidate_from_the_public_signature_api
  - viewer-core::sigflags_discovery_audit zero_sigflags_does_not_hide_a_candidate_from_the_public_signature_api
  - viewer-core::sigflags_discovery_audit document_timestamp_existence_is_examined_when_sigflags_is_explicitly_zero
  - viewer-core::sigflags_discovery_audit document_timestamp_existence_scans_signature_fields_even_without_sigflags_bit_one
- Annex Q.2 (survey), transparency conditions travel with q/Q, are set per key by `gs`, mark the page only at a
  rendering operator, and a BM array is judged by its first recognised mode (§11.3.5). ADR 2820 section 3. Tests:
  - pdf-archive survey::graphics_state::annex_q_audit_tests::a_q2_transparent_state_restored_before_painting_does_not_mark_the_page
  - pdf-archive survey::graphics_state::annex_q_audit_tests::q2_blend_mode_arrays_use_the_first_recognised_mode
- Annex I.2, inserting a newer PDF raises the catalog /Version (`raise_version`). ADR 2815 section 1. Test:
  - pdf-transform::annex_i_insert_version inserting_a_newer_pdf_uses_the_newer_version
- §7.9.4, producer dates are checked with `Date::parse` (whole clause), superseding ADR 0855's alphabet check.
  ADR 2815 section 3. Tests:
  - pdf-transform::date_spec_audit metadata_editing_does_not_publish_an_incomplete_numeric_field
  - pdf-transform::date_spec_audit metadata_editing_does_not_publish_an_out_of_range_month
- Table 349, /ModDate is kept when the catalog has /PieceInfo; an update asking for its removal is refused. ADR
  2815 section 4. Test:
  - pdf-transform::metadata_requirements_audit pieceinfo_keeps_required_moddate_when_update_requests_removal
- Table 45, an associated file needs a ModDate; the archive route refuses without one, superseding ADR 1270's
  omission. ADR 2820 section 2. Test:
  - pdf-transform::archive associated_files_date_audit::associated_archive_file_does_not_omit_the_table45_required_modification_date
- Tiling-pattern resources in the archive reader-resource repair: a resourceless pattern gets an empty dictionary
  and supply defines its missing names there, superseding ADR 2500 item 1 for patterns. ADR 2820 section 1.
  Tests:
  - pdf-transform::archive reader_resources::pattern_resource_audit::part2_pattern_resource_repair_preserves_the_readers_marks_or_refuses
  - pdf-transform::archive reader_resources::pattern_resource_audit::part4_pattern_resource_repair_preserves_the_readers_marks_or_refuses
- Table 232 /MaxLen is enforced in every native text control (GTK Entry, PasswordEntry, TextView; Qt
  QPlainTextEdit); zero refuses every insertion. ADR 2816 section 1, superseding a point of ADR 0548. Tests:
  - viewer-gtk controls::max_len_audit::multiline_entry_obeys_maxlen_and_remains_unlimited_when_unspecified
  - viewer-gtk controls::max_len_audit::password_entry_obeys_maxlen_and_remains_unlimited_when_unspecified
  - viewer-gtk controls::max_len_audit::zero_maxlen_rejects_insertion_instead_of_using_gtk_zero_as_unlimited
- Table 233 bit 27 CommitOnSelChange: a choice commit runs /V (model `Entered::Chosen`, Qt `CommitField`, GTK
  commit on selection). ADR 2816 section 2, superseding the premise of ADR 2707. Tests:
  - viewer-qt host::tests::choice_commit_audit::an_explicit_choice_commit_runs_the_acroform_validate_action
  - viewer-qt host::tests::choice_commit_audit::commit_on_selection_runs_the_acroform_validate_action_in_the_qt_choice_callback
- Table 249, an imported /Opt replaces the target's options and reaches the form control before saving. ADR 2816
  section 3, superseding a point of ADR 1661. Tests:
  - pdf-model::fdf_field_dictionary_audit imported_choice_options_replace_the_target_options
  - pdf-model::fdf_field_dictionary_audit imported_choice_options_reach_the_form_control_before_saving
- Table 251, one FDF page dictionary is one page (templates are layered, not multiplied). ADR 2816 section 4,
  superseding a point of ADR 0091. Test:
  - pdf-model::fdf_template_count_audit template_array_members_do_not_multiply_the_fdf_page_count
- Table 248, FDF scripts run in order (Doc, Before, fields, After, AfterPermsReady) through the confined runner.
  ADR 2816 section 5, superseding a point of ADR 0090/0092. Test:
  - viewer-core::fdf_script_order_audit fdf_doc_before_after_and_after_perms_scripts_reach_the_import_runner_in_order
- Not matched in the results files: none. All 72 manifest tests appear in exactly one bullet above.

## 2. Argued not a defect

Closed with the clause or the reason that closes them; the lead's second part (LEAD2) governs where it differs.

- Prose 11, NoRotate on paper (LEAD2): Table 167's NoRotate cell is unconditional ("do not rotate the annotation's
  appearance to match the rotation of the page ... regardless of the page rotation"); the later "on the screen"
  sentence adds the screen case and does not remove paper. Keeping it applied on a printed sheet is the cell's
  reading.
- Prose 103, ADR 0934 watermark under page /Rotate (LEAD2): §12.5.6.22 fixes size and position "relative to the
  target media" and says nothing of /Rotate; ADR 0934's choice stands as a documented choice in a silence.
- Prose 14, FDF field with no /V (LEAD2): §12.7.6.2's IncludeNoValueFields writes a valueless field "by name only,
  with no associated value", so a field with no /V is the standard's own export of an empty field. ADR 2816
  section 6.
- Prose 55, ADR 0901's struck sentence (LEAD2): §12.5.6.18's "If AP is not present ..." sentence is struck by
  Errata Collection 3 issue 42; comments now call drawing nothing this reader's choice and cite the erratum. ADR
  2818 section 7. Behaviour unchanged.
- Prose 10, DCT (LEAD): follow-ups only (TN 5116 marker rules, an ignored progressive golden); its one documentation
  finding is item 107.
- Prose 104, `zune-benches` in the vendored README (LEAD): the `doc/zune-image` directory is not in the repository;
  an untracked checkout on the audit machine. Nothing to change.
- Prose 73 (LZW reset) and 74 (overprint) (LEAD): the audit strengthened both tests and recorded the mutations
  that now fail them. Prose 93 closed with 18 (the unit test now asserts zero fill).
- Prose 12, Link /BS dispatch (AN): direct evidence exists in `an_underline_border_is_drawn_inside_the_rectangle_...`
  and `a_border_style_dictionary_ignores_the_border_arrays_corner_radii`, both of which draw a Link from /BS.
- Prose 75, 77, 78, 79, 16, 80 (AN): closed by tests the audit strengthened or added.
- Prose 82, 84, 85, 86, 63, 90, 25 (MC): closed by the audit's added tests. Prose 92: GPU-availability skip owned
  by the GPU harness, not changed.
- Prose 17 (LEFT): §12.7.5.5 Table 255 seed values constrain a signing handler, and none exists.
- Prose 22 (LEFT): ADR 2440 retained with limits (§14.11.2.2 guidelines); no comment repeats a refuted claim.
- Prose 33 (LEFT): Table 95 makes /ID optional; mandatory-ID matching is a trust policy, `reference.rs` says so.
- Prose 37 (LEFT): ADR 0069 superseded by ADR 1765 (§8.7 shading cache); ADR 1001 historical.
- Prose 38 (LEFT): ADR 2498's blanket inline-image refusal superseded by ADR 2516 (§8.9.7).
- Prose 40 (LEFT): ADR 0135 reversed by ADR 0316 (§12.4.4.2); code cites ADR 0316.
- Prose 42 (LEFT): ADR 0674's equality is false but no rendering bug (§12.5.6.8); `appearance.rs` does not repeat it.
- Prose 46 (LEFT): a correction of the audit's own draft attribution (ADR 0049); `oracle.rs` already says so.
- Prose 56 (LEFT): corrected attributions (ADR 1035, 1535); Tables 403/404 observations are informative NOTEs.
- Prose 60 (LEFT): ADR 1247 wording only; `variable_text.rs` states the correct direction.
- Prose 67 (LEFT): ADR 1383's PIMA fallback list superseded by extended colour handling (T.801 M.9.2, §8.9.5.2).
- Prose 68 (LEFT): ADR 2154's reported save omission unchanged and still reported (Table 251).
- Prose 70 (LEFT): ADR 1473's stale-premise claim rejected by the review; ADR 2346's historical timing not rerun
  (§7.5.4 /Size).
- Prose 71 (LEFT): ADR 0237/0307/0027 refusal limits not repeated as current (§11.4.6, §11.6.4.3, §11.5).
- Prose 88 (LEFT): the undefined-AS assertion is sound for its own claim (§12.5.5 "reasonable behaviour (such as
  displaying nothing)"); Print gating is covered by the audit's flag tests.
- Prose 91, 95, 96, 102 (LEFT): closed by the audit's own test, a policy reclassification, or methodological
  qualifications of the ledger's reference instrument that name no row.

## 3. Prose findings

1. Calculator (Type 4) silent error fallbacks: fixed (LEFT, no ADR; parse-time refusal and recorded run-time
   fault; reporting at draw time left for a continuation).
2. Annex I.2 missing "change or save" warning: fixed (TR, ADR 2815 section 2).
3. Annex I first-error suppression: owner: Q834 section 4 (TR declined it, ADR 2815 section 2, trap 5).
4. XMP packet date removed on an invalid date: fixed (TR, ADR 2815 section 3).
5. Qt print bridge forwards one PrintPageRange pair: fixed (VW, ADR 2814).
6. GTK sidebar expansion cap of 4096: fixed (VW, ADR 2814).
7. Qt multiline entry has no MaxLen cap: fixed (FM, ADR 2816 section 1; no native C++ test exists).
8. Qt selection callback omits commit-on-selection: fixed (FM, ADR 2816 section 2).
9. Model MaxLen accepts an over-limit string: fixed on the native side (FM, ADR 2816 section 1); model half:
   owner: Q834 section 1.
10. DCT foreign-marker and TN 5116 follow-ups: closed: argued (LEAD; follow-ups, not a defect; documentation
    finding is item 107).
11. NoRotate on paper: closed: argued (§12.5.3 Table 167, LEAD2).
12. Link /BS dispatch unproven: closed: argued (§12.5.5 / Table 176; existing tests draw a Link from /BS, AN).
13. Soft-mask knockout false: fixed (MC, ADR 2817 section 2; residue for non-isolated blending groups remains).
14. FDF absent /V: closed: argued (§12.7.6.2 IncludeNoValueFields, LEAD2, ADR 2816 section 6).
15. Redact /RO rationale versus the overlay exclusion: owner: Q834 section 3.
16. Redaction QuadPoints test: closed: argued (closed by the audit's own test, AN).
17. Unconsumed seed constraints: closed: argued (§12.7.5.5 Table 255, LEFT).
18. ADR 0369/0371 bit shift: fixed (LEAD, ADR 2811).
19. ADR 0855 date check: fixed (TR, ADR 2815 section 3).
20. ADR 0814 Desc and ADR 1270 ModDate: fixed (MC, ADR 2812 section 1 for Desc; TR, ADR 2820 section 2 for ModDate).
21. ADR 2456 Count repair: fixed (LEAD2, ADR 2819 supersedes ADR 2456 for that point).
22. ADR 2440 retained with limits: closed: argued (§14.11.2.2, LEFT).
23. ADRs 0343/0359 prefix rationale: qualification recorded (ADR 2812 section 7; policy stands, rationale replaced).
24. ADR 0183 standard-font-name fallback: qualification recorded (ADR 2812 section 8).
25. ADR 0115 font-cache test: closed: argued (closed by the audit's new test, MC).
26. ADR 2500 pattern assignment: fixed (TR, ADR 2820 section 1).
27. ADR 0162 FitBH/FitBV: fixed (VW, ADR 2814).
28. ADR 0244 evidence for the 4096 cap: fixed (VW, ADR 2814; cap removed).
29. ADR 0025 NOTE as licence: qualification recorded (ADR 2812 section 9).
30. ADR 0017 claim too broad, ADR 1106 audience data: ADR 0017 closed: argued (§8.11.4.3, LEFT); ADR 1106 left for
    a continuation (a §8.11.4.4 reading not done).
31. ADR 0177 Link QuadPoints routing: fixed (VW, ADR 2814).
32. ADR 0123 implicit-P policy: fixed (AN, ADR 2813).
33. ADR 1101 mandatory-ID matching: closed: argued (Table 95, LEFT).
34. ADR 1175 NextPage test: fixed (VW, ADR 2814).
35. ADR 0164 PV/PI in multi-page layouts: fixed (VW, ADR 2814; PV/PI scripts still ride the open/close).
36. ADR 1602 interleaving limitation: fixed (VW, ADR 2814 section 4; only for a link's /U chain).
37. ADR 0069 and ADR 1001: closed: argued (superseded by ADR 1765, LEFT).
38. ADR 2498 inline-image refusal: closed: argued (superseded by ADR 2516, §8.9.7, LEFT).
39. ADR 0451/0456 citation, ADR 1121: fixed (MC, comment cites ADR 0451 section 2).
40. ADR 0135 premises: closed: argued (ADR 0316, §12.4.4.2, LEFT).
41. ADRs 0133, 0358, 2687 retired premises: qualification recorded (MC, ADR 2812 section 8).
42. ADR 0674 equality claim: closed: argued (§12.5.6.8, LEFT).
43. ADR 0168/1179/1180 NoZoom under print: fixed (AN, ADR 2813).
44. ADR 0086 and ADR 1074 stale: qualification recorded here; both ADRs are records superseded by the later `/FD` and descriptor consumers, and no live comment or ledger note repeats either claim (lead).
45. ADR 0444 half-pixel inference: fixed (LEFT; comment and todo text only, no ADR).
46. ADR 0049 attribution: closed: argued (correction of the draft, LEFT).
47. ADR 0015/0050 TrueType route: fixed (MC, ADR 2817 section 1; the two ADRs stand).
48. ADR 0192 and ADR 0075 Table 178: fixed (AN, ADR 2818).
49. ADR 1784 polyline IC: fixed (AN, ADR 2818).
50. ADR 1425 subset tags: fixed in part (AN, ADR 2818; cross-stream uniqueness left for a continuation).
51. ADR 0043 quad ordering, retrieval comment: fixed (AN, ADR 2818).
52. ADR 0304 regeneration: Widget half fixed (AN, ADR 2818); plain FreeText half left for a continuation.
53. ADR 1622 Stamp row: fixed (VW, ADR 2814).
54. ADR 0295 Contents/Desc: fixed (VW, ADR 2814).
55. ADR 0901, ADR 0109, ADR 1561: closed: argued (Errata Collection 3 issue 42, LEAD2, ADR 2818 section 7).
56. ADR 1035 LastModified, ADR 1535 TrapNet: closed: argued (informative NOTEs, LEFT).
57. ADR 0181 SigFlags: fixed (VW, ADR 2814); ADR 1216 GTK chooser half left for a continuation.
58. ADR 2707 click-commit premise: fixed (FM, ADR 2816 section 2).
59. ADR 0548 appearance-side enforcement: fixed (FM, ADR 2816 section 1).
60. ADR 1247 determinant wording: closed: argued (wording only, LEFT).
61. ADR 0582 pointwise union versus raster averaging: qualification recorded; the limitation stands with §10.7.4's anti-aliasing departure, and `render-raster/tests/abutting_marks.rs`'s header no longer presents the seam as §11.3.7.3's (lead).
62. ADR 0047 blend.rs header: fixed (MC, comment).
63. ADR 1107 nonknockout fixture: closed: argued (closed by the audit's new test, MC).
64. ADR 1661 imported /Opt: fixed (FM, ADR 2816 section 3).
65. ADR 0090/0092 FDF scripts: fixed (FM, ADR 2816 section 5).
66. ADR 0091 FDF Templates and named-page Type: fixed (FM, ADR 2816 section 4; MC, named_page.rs); Table 253 /F
    templates left for a continuation.
67. ADR 1383 PIMA fallback list: closed: argued (T.801 M.9.2, §8.9.5.2, LEFT).
68. ADR 2154 save omission: closed: argued (Table 251, LEFT).
69. ADR 2508 knockout:false boundary: fixed (MC, ADR 2817 section 2).
70. ADRs 1473/1212 census: closed: argued (§7.5.4 /Size, LEFT).
71. ADR 0455/0237/0307/0027 limits: closed: argued (§11.4.6, §11.6.4.3, §11.5, LEFT).
72. 16-bit sample test survives byte reversal: fixed (MC; test strengthened, plant fails).
73. LZW reset assertion: closed: argued (strengthened by the audit, LEAD).
74. Overprint inverted stroking flag: closed: argued (strengthened by the audit, LEAD).
75. Absent-Group knockout default: closed: argued (strengthened by the audit, AN).
76. `none_removes_the_mask` relies on recovery: fixed (MC; exact xref asserted, masked control added).
77. Text-annotation implicit NoRotate: closed: argued (new audit rotation test, AN).
78. Collinear Ink cubic: closed: argued (new audit cubic Ink test, AN).
79. FreeText RD draft: closed: argued (accepted borderless FreeText RD test, AN).
80. Redaction QuadPoints bow-tie test: closed: argued (audit test uses quads smaller than Rect, AN).
81. Tx simple-pair replacement test: fixed (LEFT; nested `/Span` added, plant fails).
82. Interpolation flag test: closed: argued (closed by the audit's added test, MC).
83. "First visible" alternate fixture: fixed (LEFT; third alternate added, plant fails).
84. Alpha-mask transfer tests: closed: argued (closed by the audit's added test, MC).
85. HardLight/Overlay mapping: closed: argued (closed by the audit's absolute oracle, MC).
86. DCT canonical-marker test: closed: argued (closed by the audit's added test, MC).
87. Glyph-stroke pattern test: fixed (LEFT; tiled ink compared against solid, plant fails).
88. Undefined-AS assertion, FileAttachment spare AP: closed: argued (§12.5.5, LEFT).
89. Link/OC test varies only group visibility: qualification only; the audit's added tests cover the varied flags and the M/C precedence, and they pass; no code defect (lead).
90. ADR 1107 and ADR 1175 test weaknesses: closed: argued (with 63 and 34, MC).
91. Form dash-phase equality test: closed: argued (closed by the audit's own test, LEFT).
92. Tests that return early without a corpus or adapter: closed: argued (GPU-availability skip owned by the GPU
    harness, MC).
93. Tests that require sign extension: closed: argued (with item 18, the unit test now asserts zero fill, LEAD).
94. Page-label fixtures: fixed (VW; real 9- and 7-page documents with a page-count assertion).
95. Courier 600-versus-400 width test: closed: argued (policy evidence, EC3 #453, LEFT).
96. Ledger whole-target references: closed: argued (methodological qualification, LEFT).
97. `abutting_marks.rs` header: fixed (MC; header states ADR 0582's reading).
98. Annex I completed-row explanation: fixed (TR, ADR 2815 section 1).
99. §12.2 row label versus /Enforce and /HideMenubar: owner: Q834 section 2 (VW moved ledger row 12.2 to partial).
100. Ledger 12.5.3 attribution of EC3 #34: fixed; the 12.5.3 note now places the NOTE in §12.5.2's amended NOTE 2 (lead).
101. Soft-mask ledger row and output-intent entry-call description: fixed (MC, ledger 11.6.5.1 note).
102. Ledger reference counts: closed: argued (methodological qualification, LEFT).
103. ADR 0934 watermark under page /Rotate: closed: argued (§12.5.6.22, LEAD2).
104. `zune-benches` in the vendored README: closed: argued (directory not in the repository, LEAD).
105. PolyLine/Polygon Path/Vertices comment: fixed (AN, ADR 2818).
106. Source comment citing ADR 0456: fixed (MC).
107. Test comment attributing APP14 transform 2 to Table 13: fixed (LEAD, `dct_components.rs`; MC,
    `colour_transform.rs`).
108. `blend.rs` header repeats a withdrawn cause: fixed (MC).
109. `abutting_marks.rs` header stale: fixed (MC).
110. MCAF/MathML example file: fixed (LEFT; MIME corrected, guard removed).
111. Errata narrative misplacement of EC3 #34: fixed in `doc/errata-read.md` with item 100 (lead).
112. Missing "change or save" warning: fixed (TR, ADR 2815 section 2).

## 4. Left for a continuation

Merged from every group's "left" section; results-LEAD2 governs.

- Prose 30, second half (ADR 1106): missing audience data outside the normative comparison. A §8.11.4.4 reading is
  not done.
- Prose 52 (ADR 0304): plain FreeText whole regeneration. A §12.7.4.3 and §12.5.6.6 reading is not done.
- Table 196 mixed order is interleaved only for a link's /U chain; outline items, page /O and /C, the OpenAction
  chain and other annotation events still run scripts and non-script actions in two passes (ADR 2814 section 4).
- Table 197 /PV and /PI scripts still ride the current page's open and close; only their non-script actions follow
  visibility (ADR 2814 section 3).
- Prose 57, second half: ADR 1216's GTK chooser absence was not looked at.
- §7.10.5.3 run-time faults in tint transforms, transfer functions, black generation, mesh shadings and the type 1
  shading grid are recorded on the function but not yet reported; wiring a report needs the content interpreter
  to ask `evaluation_fault()` after the page. The GPU `device_program` evaluator records nothing.
- §9.9.2 subset-tag uniqueness across the writers of one saved file (needs a registry across the six `for_a_file`
  calls). Ledger rows 9.9.2 and 9.9 are `partial` meanwhile.
  Round 2007: a save now names every subset through one record for the file, against the names the file already
  states (ADR 2832); the archive conversion's writers and FDF-imported appearances remain, so both rows stay
  `partial`. The §7.10.5.3 item above is set out, unbuilt, in ADR 2833.
- Table 253 /F: FDF templates naming another file still arrive one page each on supply.
- Composition of several templates on one FDF page: first template's page, all templates as forms in array order
  (ADR 2816 section 4); the owner may prefer otherwise.
- 11.6.5.1 residue: non-isolated knockout mask groups whose elements blend, and four-component mask groups, are
  still reported.
- /Alt and /E phrases with a mid-string language escape are still spoken in one language (ADR 2812 section 2).
- Table 178 /LE with /LL: the literal reading was implemented; the other tenable reading (endings on the drawn
  line) is recorded in ADR 2818 section 2.
- Windows host (viewer-ffi): whether the C# side commits on selection when bit 27 is set was not checked.
- Qt C++ changes (QPageRanges in `window.cpp`, the multiline MaxLen cap) have no native test; the page-range change
  was not compiled by its author.
- Open consequences the owner should know of: issue19634.pdf now draws unblurred and unmasked (ADR 2812 section 3);
  base images whose /Decode length disagrees with the colour space are refused (ADR 2812 section 5), so those corpus
  pages leave the judged set and should be counted in the next corpus walk.
- The four owner questions are doc/questions/Q834: (1) a model-side /MaxLen cut, which contradicts two passing audit
  tests; (2) §12.2 /Enforce level; (3) Redact /RO against the overlay exclusion; (4) Annex I first-error suppression.
  Round 2011: A834 built (ADRs 2840, 2841): MaxLen cut, /Enforce warn, removal proven, Annex I pinned; /RO is Q842.

## 5. Files touched

By crate, merged from the groups' lists. Documents are listed last.

- pdf-syntax: `src/filter.rs`, `src/write.rs`, `src/document.rs`, `src/document/objects.rs`,
  `src/document/repair.rs` and the files of the generation census (`defined_id` call sites), `src/text_string.rs`,
  `src/lib.rs`.
- pdf-colour: `src/function.rs`, `src/shading.rs`.
- pdf-font: `src/loading/opening.rs`.
- pdf-model src: `action.rs`, `annotation.rs`, `appearance.rs`, `attachment.rs`, `attachment/filing.rs`,
  `content.rs`, `content/annotations.rs`, `content/ext_gstate.rs`, `content/font.rs`, `content/marked.rs`,
  `content/pattern.rs`, `content/pattern/tiling.rs`, `content/resources.rs`, `content/run/compat_type3_ops.rs`,
  `content/run/marked_ops.rs`, `content/run/nested.rs`, `content/text/type3.rs`,
  `content/transparency/soft_mask.rs`, `forms_data.rs`, `image/dispatch.rs`, `image/mask.rs`, `image/space.rs`,
  `link.rs`, `named_page.rs`, `popup.rs`, `retrieval.rs`, `soft_mask.rs`, `variable_text.rs`, `view.rs`,
  `view/annotations.rs`, `view/fields.rs`, `view/markup.rs`, `view/pages.rs`, `view/script_sites.rs`,
  `view/scripts.rs`, `view/writing.rs`, `xfdf.rs`.
- pdf-model tests: `annotations.rs` (six tests updated to ADR 2813/2818), `associated_file_examples.rs`,
  `bit_depths.rs`, `colour_transform.rs`, `damaged_content_streams.rs`, `dct_components.rs`,
  `form_limits_and_commits.rs`, `optional_content.rs`, `page_labels.rs`, `shadings.rs`, `soft_masks.rs`,
  `tiling.rs`, `variable_text.rs`.
- pdf-render: `src/paint.rs`, `src/edge.rs`, `src/sub_pixel.rs`. render-cpu: `src/blend.rs`. render-raster:
  `tests/abutting_marks.rs`.
- pdf-signature: `src/signature.rs`.
- pdf-archive: `src/survey.rs`, `src/survey/graphics_state.rs`, `src/survey/operators.rs`, `src/table/graphics.rs`,
  and the coverage entries and row for the page-tree Count (ADR 2819).
- pdf-transform: `src/update.rs`, `src/archive/attach.rs`, `src/archive/reader_resources.rs`,
  `src/archive/page_tree.rs` (page Count restatement), `tests/archive.rs`; `optimize` and `redact` refuse a source
  whose Count differs from its leaves.
- viewer-core: `src/interact.rs`, `src/notes.rs`, `src/open.rs`, `src/open/destinations.rs`, `src/viewer.rs`,
  `src/viewer/answers.rs`, `src/viewer/layout.rs`, `src/viewer/navigation.rs`, `src/viewer/outcomes.rs`,
  `tests/headless.rs`.
- viewer-host: `src/documents.rs`, `src/panel.rs`.
- viewer-gtk: `src/tree.rs`, `src/controls.rs`, `src/controls/selection_commits.rs`.
- viewer-qt: `src/bridge.rs`, `src/host/printing.rs`, `src/host/forms.rs`, `cpp/window.cpp`.
- viewer-device: `src/stale/placement.rs` (comment only).
- doc: `doc/conformance/ledger.toml` (rows 7.9.4, 7.10.5.3, 9.9, 9.9.2, 11.6.5.1, 12.2, 12.3.2.2, 12.5.6.5,
  12.5.6.7, 12.5.6.9, 12.5.6.10, 12.5.6.19, 12.7.5.3, 12.7.5.4, 12.7.8.3.1 to 12.7.8.3.3, 14.13.2, I.2, Q.2, and
  the page-tree Count row), `doc/todo/37-a-frame-that-says-it-is-stale.md`, `doc/todo/65-the-remaining-frontier.md`,
  ADRs 2811 to 2820, and `doc/questions/Q834`.

## 6. What the fixes moved in tier 2, against HEAD

- `pdf-model` `corpus`: 61 documents draw incompletely, HEAD 62. `issue19389.pdf` left the list: regeneration keeps a
  widget's stored `/Resources` over `/DR`, and its `/DA` font is defined there (§12.7.4.3, ADR 2818 section 5). The
  two font-face joiners, `issue15053.pdf` and `mixedfonts.pdf`, are HEAD's on this machine.
- `pdf-model` `raster_golden`: 23 first pages move, the 16 HEAD moves on this machine and seven more.
  `annotation-squiggly-without-appearance.pdf` and `annotation-underline-without-appearance.pdf`: a text markup
  quadrilateral read as four corners, bottom edge first (§12.5.6.10, ADR 2818 section 4). `issue19634.pdf`: Table
  111's `d1` ignores the glyph's colour-related parameters, its soft mask included (ADR 2812). `issue19389.pdf`: above.
  `bug1538111.pdf`, `bug1703683_page2_reduced.pdf` and `issue14565.pdf`: cause not traced in this round; the first
  thing a continuation reads.
