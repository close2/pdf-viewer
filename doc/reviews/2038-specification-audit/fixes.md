# The queue of review 2038's findings, and what each round did with them

Review 2038 (`doc/reviews/2038-specification-audit.md`, merged by round 2070) left 318 findings in
`findings.json`: 78 behaviour findings (B), 172 test findings (T) and 68 documentation findings (D). This
file is the queue the audit-fix rounds work from: one line per finding, in groups that share a cause.

**A group is a work unit.** Two groups name disjoint production source files, so two rounds taking two
groups never edit the same source file; `doc/conformance/ledger.toml` and the ADR directory are the only
files every group may touch. A group's files were located by grepping for the function or concept the
finding names; a path ending in `?` was matched by subject only, and the round that takes the group
confirms it first. A T finding joins the group of the first file it names and does not join groups
together, because most T findings are a test the audit already added and that passes: their work is to
check what the finding's `remaining` field leaves, not to change source.

Groups are ordered by the crate most of their findings concern, in the dependency order pdf-syntax,
pdf-colour, pdf-font, pdf-model, pdf-signature, pdf-transform, pdf-archive, the renderers, pdf-script,
the viewer crates, then the documents. G001 is the group the owner's work order names first: B54, B07,
B21, B57, B09, B10 and B11, with the T and D findings in the same files.

Line shape: `id | crates | kind (from findings.json) | status | cause | failures in expected-failures.json`.
Status is `open`, `fixed (ADR, tests)`, `argued (clause, ADR)` or `question (Q)`. What fails on the merged
tree is `expected-failures-current.json`; every audit-fix round compares its nextest against that file.

## Counts per crate

| crate | groups | B | T | D |
|---|---|---|---|---|
| pdf-syntax | 8 | 9 | 20 | 5 |
| pdf-colour | 2 | 1 | 4 | 0 |
| pdf-font | 6 | 5 | 7 | 1 |
| pdf-model | 62 | 30 | 104 | 21 |
| pdf-signature | 1 | 4 | 2 | 0 |
| pdf-transform | 8 | 4 | 5 | 2 |
| pdf-archive | 3 | 2 | 2 | 2 |
| pdf-ccitt | 1 | 0 | 1 | 0 |
| render-cpu | 2 | 4 | 3 | 0 |
| render-raster | 2 | 0 | 2 | 0 |
| render-gpu | 2 | 0 | 1 | 1 |
| pdf-script | 2 | 1 | 1 | 0 |
| viewer-core | 11 | 15 | 12 | 3 |
| viewer-host | 2 | 2 | 2 | 0 |
| viewer-accessibility | 4 | 1 | 1 | 2 |
| viewer-ui | 3 | 0 | 2 | 1 |
| viewer-confined | 1 | 0 | 1 | 0 |
| viewer-gtk | 1 | 0 | 1 | 0 |
| confined-transport | 1 | 0 | 1 | 0 |
| doc | 21 | 0 | 0 | 30 |

## Index of the groups

| group | crate | B | T | D |
|---|---|---|---|---|
| G001 | pdf-syntax | 8 | 12 | 3 |
| G002 | pdf-syntax | 1 | 1 | 1 |
| G003 | pdf-syntax | 0 | 2 | 1 |
| G004 | pdf-syntax | 0 | 1 | 0 |
| G005 | pdf-syntax | 0 | 1 | 0 |
| G006 | pdf-syntax | 0 | 1 | 0 |
| G007 | pdf-syntax | 0 | 1 | 0 |
| G008 | pdf-syntax | 0 | 1 | 0 |
| G009 | pdf-colour | 1 | 1 | 0 |
| G010 | pdf-colour | 0 | 3 | 0 |
| G011 | pdf-font | 3 | 2 | 0 |
| G012 | pdf-font | 2 | 1 | 0 |
| G013 | pdf-font | 0 | 2 | 0 |
| G014 | pdf-font | 0 | 0 | 1 |
| G015 | pdf-font | 0 | 1 | 0 |
| G016 | pdf-font | 0 | 1 | 0 |
| G017 | pdf-model | 10 | 11 | 2 |
| G018 | pdf-model | 3 | 1 | 0 |
| G019 | pdf-model | 3 | 0 | 0 |
| G020 | pdf-model | 2 | 2 | 1 |
| G021 | pdf-model | 1 | 8 | 0 |
| G022 | pdf-model | 1 | 3 | 1 |
| G023 | pdf-model | 1 | 1 | 2 |
| G024 | pdf-model | 1 | 2 | 0 |
| G025 | pdf-model | 1 | 1 | 1 |
| G026 | pdf-model | 1 | 2 | 0 |
| G027 | pdf-model | 1 | 1 | 0 |
| G028 | pdf-model | 1 | 1 | 0 |
| G029 | pdf-model | 1 | 0 | 0 |
| G030 | pdf-model | 1 | 0 | 0 |
| G031 | pdf-model | 1 | 0 | 0 |
| G032 | pdf-model | 1 | 0 | 0 |
| G033 | pdf-model | 0 | 0 | 6 |
| G034 | pdf-model | 0 | 6 | 0 |
| G035 | pdf-model | 0 | 3 | 1 |
| G036 | pdf-model | 0 | 4 | 0 |
| G037 | pdf-model | 0 | 4 | 0 |
| G038 | pdf-model | 0 | 2 | 1 |
| G039 | pdf-model | 0 | 2 | 1 |
| G040 | pdf-model | 0 | 3 | 0 |
| G041 | pdf-model | 0 | 3 | 0 |
| G042 | pdf-model | 0 | 3 | 0 |
| G043 | pdf-model | 0 | 3 | 0 |
| G044 | pdf-model | 0 | 3 | 0 |
| G045 | pdf-model | 0 | 1 | 1 |
| G046 | pdf-model | 0 | 2 | 0 |
| G047 | pdf-model | 0 | 2 | 0 |
| G048 | pdf-model | 0 | 2 | 0 |
| G049 | pdf-model | 0 | 2 | 0 |
| G050 | pdf-model | 0 | 2 | 0 |
| G051 | pdf-model | 0 | 0 | 1 |
| G052 | pdf-model | 0 | 0 | 1 |
| G053 | pdf-model | 0 | 0 | 1 |
| G054 | pdf-model | 0 | 0 | 1 |
| G055 | pdf-model | 0 | 1 | 0 |
| G056 | pdf-model | 0 | 1 | 0 |
| G057 | pdf-model | 0 | 1 | 0 |
| G058 | pdf-model | 0 | 1 | 0 |
| G059 | pdf-model | 0 | 1 | 0 |
| G060 | pdf-model | 0 | 1 | 0 |
| G061 | pdf-model | 0 | 1 | 0 |
| G062 | pdf-model | 0 | 1 | 0 |
| G063 | pdf-model | 0 | 1 | 0 |
| G064 | pdf-model | 0 | 1 | 0 |
| G065 | pdf-model | 0 | 1 | 0 |
| G066 | pdf-model | 0 | 1 | 0 |
| G067 | pdf-model | 0 | 1 | 0 |
| G068 | pdf-model | 0 | 1 | 0 |
| G069 | pdf-model | 0 | 1 | 0 |
| G070 | pdf-model | 0 | 1 | 0 |
| G071 | pdf-model | 0 | 1 | 0 |
| G072 | pdf-model | 0 | 1 | 0 |
| G073 | pdf-model | 0 | 1 | 0 |
| G074 | pdf-model | 0 | 1 | 0 |
| G075 | pdf-model | 0 | 1 | 0 |
| G076 | pdf-model | 0 | 1 | 0 |
| G077 | pdf-model | 0 | 1 | 0 |
| G078 | pdf-model | 0 | 1 | 0 |
| G079 | pdf-signature | 4 | 2 | 0 |
| G080 | pdf-transform | 1 | 1 | 0 |
| G081 | pdf-transform | 1 | 1 | 0 |
| G082 | pdf-transform | 1 | 0 | 0 |
| G083 | pdf-transform | 1 | 0 | 0 |
| G084 | pdf-transform | 0 | 1 | 1 |
| G085 | pdf-transform | 0 | 0 | 1 |
| G086 | pdf-transform | 0 | 1 | 0 |
| G087 | pdf-transform | 0 | 1 | 0 |
| G088 | pdf-archive | 2 | 1 | 1 |
| G089 | pdf-archive | 0 | 0 | 1 |
| G090 | pdf-archive | 0 | 1 | 0 |
| G091 | pdf-ccitt | 0 | 1 | 0 |
| G092 | render-cpu | 2 | 2 | 0 |
| G093 | render-cpu | 2 | 1 | 0 |
| G094 | render-raster | 0 | 1 | 0 |
| G095 | render-raster | 0 | 1 | 0 |
| G096 | render-gpu | 0 | 0 | 1 |
| G097 | render-gpu | 0 | 1 | 0 |
| G098 | pdf-script | 1 | 0 | 0 |
| G099 | pdf-script | 0 | 1 | 0 |
| G100 | viewer-core | 7 | 7 | 0 |
| G101 | viewer-core | 3 | 0 | 1 |
| G102 | viewer-core | 1 | 2 | 0 |
| G103 | viewer-core | 1 | 0 | 1 |
| G104 | viewer-core | 1 | 0 | 0 |
| G105 | viewer-core | 1 | 0 | 0 |
| G106 | viewer-core | 1 | 0 | 0 |
| G107 | viewer-core | 0 | 0 | 1 |
| G108 | viewer-core | 0 | 1 | 0 |
| G109 | viewer-core | 0 | 1 | 0 |
| G110 | viewer-core | 0 | 1 | 0 |
| G111 | viewer-host | 2 | 1 | 0 |
| G112 | viewer-host | 0 | 1 | 0 |
| G113 | viewer-accessibility | 1 | 0 | 0 |
| G114 | viewer-accessibility | 0 | 0 | 1 |
| G115 | viewer-accessibility | 0 | 0 | 1 |
| G116 | viewer-accessibility | 0 | 1 | 0 |
| G117 | viewer-ui | 0 | 0 | 1 |
| G118 | viewer-ui | 0 | 1 | 0 |
| G119 | viewer-ui | 0 | 1 | 0 |
| G120 | viewer-confined | 0 | 1 | 0 |
| G121 | viewer-gtk | 0 | 1 | 0 |
| G122 | confined-transport | 0 | 1 | 0 |
| G123 | doc | 0 | 0 | 5 |
| G124 | doc | 0 | 0 | 5 |
| G125 | doc | 0 | 0 | 2 |
| G126 | doc | 0 | 0 | 1 |
| G127 | doc | 0 | 0 | 1 |
| G128 | doc | 0 | 0 | 1 |
| G129 | doc | 0 | 0 | 1 |
| G130 | doc | 0 | 0 | 1 |
| G131 | doc | 0 | 0 | 1 |
| G132 | doc | 0 | 0 | 1 |
| G133 | doc | 0 | 0 | 1 |
| G134 | doc | 0 | 0 | 1 |
| G135 | doc | 0 | 0 | 1 |
| G136 | doc | 0 | 0 | 1 |
| G137 | doc | 0 | 0 | 1 |
| G138 | doc | 0 | 0 | 1 |
| G139 | doc | 0 | 0 | 1 |
| G140 | doc | 0 | 0 | 1 |
| G141 | doc | 0 | 0 | 1 |
| G142 | doc | 0 | 0 | 1 |
| G143 | doc | 0 | 0 | 1 |

## pdf-syntax

### G001 — algorithm 2.B round cap; crypt filter Identity entry parsed before ignore rule; crypt filter selector defaults; deliberate departure: xref entries above Size; function degenerate domain arithmetic; image Decode array element validation; indirect DecodeParms values not resolved; password preprocessing r6 vectors; perms-over-P tamper authority claim; public-key handler StmF/StrF recipients; test control: LZW EarlyChange default; test control: Prev chain traversal; test control: indirect values need resolving at consumer; test control: object-zero free list; test control: uncompressed ObjStm parameters; test oracle independent of production helper; type4 calculator NUL whitespace; xref module doc overclaims valid-file coverage; xref recovery and free-entry handling; xref recovery revives freed objects

- crates: pdf-colour, pdf-model, pdf-syntax
- source files: crates/pdf-colour/src/function.rs, crates/pdf-model/src/image/dispatch.rs, crates/pdf-model/src/image/space.rs, crates/pdf-syntax/src/crypt.rs, crates/pdf-syntax/src/crypt/public_key.rs, crates/pdf-syntax/src/filter.rs, crates/pdf-syntax/src/xref.rs
- test files: crates/pdf-colour/tests/calculator_functions_spec_audit.rs, crates/pdf-colour/tests/functions_spec_audit.rs, crates/pdf-colour/tests/sampled_functions_spec_audit.rs, crates/pdf-model/tests/decode_element_types_audit.rs, crates/pdf-model/tests/indirect_function_colour_spec_audit.rs, crates/pdf-syntax/tests/encryption.rs?, crates/pdf-syntax/tests/encryption_fixtures.rs, crates/pdf-syntax/tests/incremental_object_zero_spec_audit.rs, crates/pdf-syntax/tests/indirect_filter_parameters_spec_audit.rs, crates/pdf-syntax/tests/lzw_full_table_audit.rs, crates/pdf-syntax/tests/prefixed_incremental_xref_spec_audit.rs, crates/pdf-syntax/tests/prev_xref_stream_spec_audit.rs, crates/pdf-syntax/tests/public_key_witnesses.rs, crates/pdf-syntax/tests/spec_objxref_audit.rs, crates/pdf-syntax/tests/type0_deletion_spec_audit.rs

- 2038-B07 | pdf-syntax | behavior-gap | fixed (§7.6.2 Table 20 `/CF`; `crypt_filters` skips a `/CF` key Table 26 lists before reading it; encryption_fixtures::a_cf_entry_named_identity_is_ignored_even_if_its_cfm_is_unknown passes) | crypt filter Identity entry parsed before ignore rule | 1 manifest failure
- 2038-B09 | pdf-colour | behavior-gap | fixed (ADR 2963; stitching compares Bounds 0 with Domain 0 by IEEE `==`; functions_spec_audit::stitching_treats_signed_zero_domain_and_bound_as_the_same_degenerate_endpoint passes) | function degenerate domain arithmetic | 1 manifest failure
- 2038-B10 | pdf-colour | behavior-gap | fixed (ADR 2963; `interpolate` is degenerate only at zero width; sampled_functions_spec_audit::sampled_interpolation_preserves_a_small_but_nonzero_domain_span passes) | function degenerate domain arithmetic | 1 manifest failure
- 2038-B11 | pdf-model | behavior-gap | fixed (Table 87 "An array of numbers"; `dispatch::inconsistent_entries` refuses a non-number element; decode_element_types_audit::decode_pairs_must_each_be_numbers passes) | image Decode array element validation | 1 manifest failure
- 2038-B21 | pdf-syntax | behavior-mismatch | fixed (ADR 2960; `xref::read` looks for a header past the window and reads the stated chain from it before rebuilding; prefixed_incremental_xref_spec_audit::a_long_header_prefix_does_not_resurrect_an_object_deleted_by_the_latest_section passes) | xref recovery revives freed objects | 1 manifest failure
- 2038-B45 | pdf-colour | calculator-lexical-whitespace | fixed (§7.2.3 Table 1; the Type 4 tokenizer splits on `pdf_syntax::lexer::is_whitespace`, NUL included; calculator_functions_spec_audit::type4_null_separates_operands_and_operators_like_the_other_five_white_spaces passes) | type4 calculator NUL whitespace | 1 manifest failure
- 2038-B54 | pdf-syntax | behavior-gap | fixed (§7.3.10; `Document::decode_parms` resolves each indirect value of the parameter dictionary once, streams excepted; both indirect_filter_parameters_spec_audit numeric tests pass) | indirect DecodeParms values not resolved | 2 manifest failures
- 2038-B57 | pdf-syntax | behavior-gap | fixed (ADR 2959; a `/StrF` filter other than `/StmF`'s keys the strings from its own seed and lists; public_key_witnesses::separate_stmf_and_strf_defaults_use_their_own_recipient_keys passes) | public-key handler StmF/StrF recipients | 1 manifest failure
- 2038-T42 | pdf-syntax | independent-oracle-gap | fixed (audit test crypt::tests::r6_password_preparation_matches_saslprep_vectors_and_byte_limit, killed by a plant dropping SASLprep; `remaining` is a scope limit) | password preprocessing r6 vectors
- 2038-T69 | pdf-syntax | test-coverage-gap | fixed (audit test lzw_full_table_audit::full_lzw_table_clear_resets_width_and_dictionary_for_both_earlychange_values, killed by a plant defaulting EarlyChange to 0) | test control: LZW EarlyChange default
- 2038-T72 | pdf-syntax | test-coverage-gap | fixed (audit test cross_references::object_stream_dictionary_values_may_reference_uncompressed_objects, killed by a plant ignoring an indirect /Length) | test control: uncompressed ObjStm parameters
- 2038-T73 | pdf-syntax | deliberate-departure-test-specificity | fixed (audit test cross_references::an_explicit_current_entry_past_size_is_retained_by_the_chosen_departure, killed by a plant dropping entries at or above /Size 4) | deliberate departure: xref entries above Size
- 2038-T74 | pdf-syntax | test-coverage-gap | fixed (audit tests indirect_filter_parameters_spec_audit::indirect_filter_name_and_decodeparms_dictionary_match_direct_values and ::indirect_filter_and_decodeparms_arrays_resolve_each_stage_value, each killed by a plant leaving its reference unresolved) | test control: indirect values need resolving at consumer
- 2038-T76 | pdf-colour | test-coverage-gap | fixed (audit test indirect_function_colour_spec_audit::sampled_function_indirect_arrays_and_numeric_members_keep_the_direct_result, killed by a plant reading array members without resolving) | test control: indirect values need resolving at consumer
- 2038-T90 | pdf-syntax | test-coverage-gap | fixed (audit test prev_xref_stream_spec_audit::valid_incremental_xref_stream_prev_can_name_a_prior_xref_stream, killed by a plant that stops at the first section) | test control: Prev chain traversal
- 2038-T91 | pdf-syntax | test-coverage-gap | fixed (audit test incremental_object_zero_spec_audit::incremental_object_zero_subsection_is_read_without_recovery, killed by a plant dropping classic free entries) | test control: object-zero free list
- 2038-T108 | pdf-syntax | test-oracle-gap | fixed (audit test serialize_encrypted::r6_writer_entries_match_an_independent_algorithms_8_to_10_vector, killed by a plant zeroing the /Perms filler; the `remaining` weak /ID assertion is strengthened in serialize_encrypted::the_plaintext_is_not_in_the_bytes, two equal 16-byte strings, killed by a plant writing a 32-byte identifier) | test oracle independent of production helper
- 2038-T130 | pdf-syntax | assertion-strengthening | fixed (audit test encryption_fixtures::absent_stream_and_string_defaults_are_identity, killed by a plant defaulting an absent selector to AESV2) | crypt filter selector defaults
- 2038-T145 | pdf-syntax | fixture-validity-and-coverage | fixed (audit test type0_deletion_spec_audit::type_zero_with_valid_width_in_hybrid_update_deletes_the_old_entry, killed by a plant making a type 0 entry no opinion) | xref recovery and free-entry handling
- 2038-T157 | pdf-syntax | assertion-quality | fixed (audit test encryption_fixtures::a_crypt_filter_with_no_name_uses_identity_instead_of_stmf, killed by a plant defaulting an unnamed /Crypt to /StmF) | crypt filter selector defaults
- 2038-D16 | pdf-syntax | source-interpretation-qualification | fixed (ADR 2962; `crypt::perms_block`'s doc says reading /Perms over a disagreeing /P is this reader's recovery policy, step (f) "shall match", Algorithm 13 "should match") | perms-over-P tamper authority claim
- 2038-D18 | pdf-syntax | implementation-bound-risk | fixed (ADR 2961; the 256-round ceiling is removed, the clause's own condition ends the loop after at most 287 repetitions) | algorithm 2.B round cap
- 2038-D42 | pdf-syntax | test-claim-qualification | fixed (the audit corrected cross_references.rs's module comment; confirmed it now separates valid-file rules from recovery and departures) | xref module doc overclaims valid-file coverage

### G002 — file identifier doc; file identifier test gap; linearized header below EC3 version floor

- crates: pdf-syntax, pdf-transform
- source files: crates/pdf-syntax/src/serialize.rs
- test files: crates/pdf-syntax/tests/serialize.rs, crates/pdf-transform/tests/linearize.rs

- 2038-B55 | pdf-syntax | behavior-gap | fixed (ADR 2958; the linearised header is raised to 1.2, Errata Collection 3 Issue #331; both linearize::linearizing_pdf_1_*_refuses_or_raises_the_header_to_the_ec3_floor pass) | linearized header below EC3 version floor | 2 manifest failures
- 2038-T54 | pdf-syntax,pdf-transform | assertion-coverage-gap | fixed (audit test serialize::a_new_files_identifier_pair_is_equal_and_distinct_from_its_source, killed by a plant writing an unequal pair; added to §14.4's ledger row) | file identifier test gap
- 2038-D28 | pdf-syntax,pdf-transform | stale-ledger-rationale | fixed (§14.4's ledger note names `reference.rs` and `forms_data.rs::belongs_to` as the two-document comparisons) | file identifier doc

### G003 — doc: hex string non-hex byte comment; lexical contexts test gap; test weakness: parser limit checks

- crates: doc, pdf-syntax
- source files: crates/pdf-syntax/src/lexer.rs
- test files: crates/pdf-syntax/tests/lexical_context_spec_audit.rs, crates/pdf-syntax/tests/parser_limit_boundaries_audit.rs

- 2038-T18 | pdf-syntax | weak-test-oracle | fixed (audit test parser_limit_boundaries_audit::complete_array_at_capacity_succeeds_and_one_more_item_names_the_limit, killed by a plant moving the array bound by one) | test weakness: parser limit checks
- 2038-T61 | pdf-syntax | strengthened-lexical-assertions | fixed (audit test lexical_context_spec_audit::non_ascii_bytes_and_vertical_tab_remain_regular_name_bytes, killed by a plant counting VT as white space) | lexical contexts test gap
- 2038-D08 | doc,pdf-syntax | incorrect-source-attribution | fixed (`lexer::read_hex_string`'s doc separates §7.3.4.3's white-space rule from this reader's skipping of other bytes) | doc: hex string non-hex byte comment

### G004 — test fixture: hybrid xref

- crates: pdf-syntax
- source files: none (test or document only)
- test files: crates/pdf-syntax/tests/cross_references.rs?

- 2038-T09 | pdf-syntax | fixture-and-coverage-gap | fixed (audit test cross_references::a_zero_width_type_field_defaults_a_valid_incremental_entry_to_type_one, killed by a plant defaulting the type to 0; `remaining` is scope) | test fixture: hybrid xref

### G005 — linearization writer parameters

- crates: pdf-syntax
- source files: crates/pdf-syntax/src/linearize.rs
- test files: crates/pdf-transform/tests/linearize.rs

- 2038-T119 | pdf-syntax | assertion-strengthening | fixed (audit test linearize::every_value_of_the_parameter_dictionary_is_where_the_file_says, killed by a plant writing /Linearized 2) | linearization writer parameters

### G006 — stream Length framing

- crates: pdf-syntax
- source files: crates/pdf-syntax/src/document/parsing.rs?
- test files: crates/pdf-syntax/tests/cross_references.rs?

- 2038-T141 | pdf-syntax | assertion-strengthening | fixed (new filters::dl_at_an_offset_where_endstream_appears_still_does_not_frame_the_stream; the audit's DL test survived a plant framing by /DL because the endstream check recovered, the new one is killed by it) | stream Length framing

### G007 — test comment: ASCII85 alphabet

- crates: pdf-syntax
- source files: none (test or document only)
- test files: crates/pdf-syntax/tests/ascii85_runlength_boundaries_audit.rs

- 2038-T20 | pdf-syntax | test-premise-and-discrimination | fixed (audit tests ascii85_runlength_boundaries::ascii85_tilde_followed_by_anything_but_greater_than_is_an_error and ::ascii85_windowed_reader_reports_a_malformed_eod_marker, killed by a plant accepting any byte after ~ on each route) | test comment: ASCII85 alphabet

### G008 — test oracle too weak or vacuous

- crates: pdf-syntax
- source files: crates/pdf-syntax/src/document.rs?
- test files: crates/pdf-syntax/tests/on_disk.rs

- 2038-T99 | pdf-syntax | test-oracle-gap | fixed (audit test on_disk::both_read_routes_compare_the_object_at_its_defined_nonzero_generation, killed by a plant answering generation 0) | test oracle too weak or vacuous


## pdf-colour

### G009 — test control: special colour space initial values; unsupported ICC fallback skips Range clamp

- crates: pdf-colour, pdf-model
- source files: crates/pdf-colour/src/colour/parsing.rs
- test files: crates/pdf-model/tests/icc_alternate_range_spec_audit.rs, crates/pdf-model/tests/special_space_initial_tints_spec_audit.rs

- 2038-B25 | pdf-colour | behavior-mismatch | fixed (ADR 2973; `ColourSpace::IccAlternate` holds Table 65's `/Range` before the alternate; icc_alternate_range_spec_audit passes) | unsupported ICC fallback skips Range clamp | 1 manifest failure
- 2038-T31 | pdf-colour,pdf-model | test-coverage-gap | fixed (round 2074: special_space_initial_tints, indexed_rounding_initial and indexed_lab_lookup each fail on a plant — Separation initial tint 0, `entry_of` floor, palette bytes not scaled to the base range) | test control: special colour space initial values

### G010 — mesh shading vertex decoding

- crates: pdf-colour
- source files: crates/pdf-colour/src/mesh.rs
- test files: crates/pdf-model/tests/shading_continuations_spec_audit.rs, crates/pdf-model/tests/type5_function_spec_audit.rs?, crates/pdf-model/tests/type5_vertex_padding_spec_audit.rs

- 2038-T137 | pdf-colour | assertion-strengthening | fixed (round 2074: type5_low_bit_vertices_pad_to_bytes_like_the_8bit_control fails with `read_vertex`'s `align` removed) | mesh shading vertex decoding
- 2038-T138 | pdf-colour | assertion-strengthening | fixed (round 2074: the Type 7 continuation test fails with flag 2's edge reversed in `shared_edge`; a_later_overlapping_patch_paints_over_the_earlier_patch fails with the patch order reversed) | mesh shading vertex decoding
- 2038-T139 | pdf-colour | assertion-strengthening | fixed (round 2074: type5_accepts_every_table82_coordinate_and_component_width fails with 32-bit `decode_at` halved, while the padding test passes) | mesh shading vertex decoding


## pdf-font

### G011 — AGL multi-scalar glyph-name text; Mac OS Roman encoding table; MacExpert undefined slot not filled; test defect: undefined encoding slot

- crates: pdf-font, pdf-model
- source files: crates/pdf-font/src/encoding.rs
- test files: crates/pdf-font/tests/agl_sequences_spec_audit.rs, crates/pdf-font/tests/mac_os_roman_table_spec_audit.rs, crates/pdf-model/tests/simple_font_standard_fill_spec_audit.rs, crates/pdf-model/tests/truetype_standard_fill_spec_audit.rs

- 2038-B15 | pdf-font | behavior-gap | fixed (ADR 2970; `text_for` reads AGL rows and `uni` groups as strings; pdf-font and viewer-core agl_sequences_spec_audit pass) | AGL multi-scalar glyph-name text | 4 manifest failures
- 2038-B16 | pdf-font | behavior-policy-conflict | fixed (ADR 2970 supersedes the whole-name refusal; an_unrecognized_component_contributes_an_empty_string passes; the unit test asserting `f_f_notaglyph` → None now asserts `ff`) | AGL multi-scalar glyph-name text | 1 manifest failure
- 2038-B50 | pdf-font | truetype-standard-encoding-fill | fixed (ADR 2971, read against ADRs 2948 and 2956, which are about name-keyed programs and stand; `truetype::fill_dictionary_gaps_from_standard`; truetype_standard_fill_spec_audit passes) | MacExpert undefined slot not filled | 1 manifest failure
- 2038-T36 | pdf-model,pdf-font | test-discrimination-gap | fixed (round 2074: simple_truetype_differences_fill_undefined_slot_from_standard_encoding fails with the absent-base fallback planted as `WinAnsiEncoding`) | test defect: undefined encoding slot
- 2038-T151 | pdf-font | assertion-quality | fixed (round 2074: fails with Table 113's `notequal` row moved to 174) | Mac OS Roman encoding table

### G012 — ToUnicode map code length and sequences; test control: ToUnicode UseCMap

- crates: pdf-font
- source files: crates/pdf-font/src/loading/codes.rs?, crates/pdf-font/src/tounicode.rs
- test files: crates/pdf-model/tests/tounicode_bfrange_sequence_destinations_spec_audit.rs, crates/pdf-model/tests/tounicode_keeps_source_code_length_spec_audit.rs, crates/pdf-model/tests/tounicode_usecmap_stream_inheritance_spec_audit.rs

- 2038-B27 | pdf-font | behavior-defect | fixed (ADR 2972; statements keyed by code length; tounicode_keeps_source_code_length_spec_audit passes; `tounicode::code_length_tests`) | ToUnicode map code length and sequences | 1 manifest failure
- 2038-B28 | pdf-font | behavior-defect | fixed (ADR 2972; a sequence `bfrange` increments its last byte; tounicode_bfrange_sequence_destinations_spec_audit passes; `tounicode::code_length_tests`) | ToUnicode map code length and sequences | 1 manifest failure
- 2038-T30 | pdf-font | test-coverage-gap | fixed (round 2074: the audit test passes with a stream-valued `/UseCMap` ignored, because its base maps CID 36 to the `A` the program gives anyway; new tounicode_usecmap_stream_base maps the base to U+0416 and fails on that plant) | test control: ToUnicode UseCMap

### G013 — test control: indirect values need resolving at consumer

- crates: pdf-font
- source files: crates/pdf-font/src/lib.rs?
- test files: crates/pdf-font/tests/cid_metrics_indirect_spec_audit.rs, crates/pdf-font/tests/simple_metrics_indirect_spec_audit.rs, crates/pdf-model/tests/indirect_differences_spec_audit.rs

- 2038-T78 | pdf-font | test-coverage-gap | fixed (round 2074: simple_metrics_indirect fails with `/Widths` members unresolved, cid_metrics_indirect with `metrics::resolved` borrowing a reference) | test control: indirect values need resolving at consumer
- 2038-T83 | pdf-font | test-coverage-gap | fixed (round 2074: indirect_integer_and_name_objects_preserve_differences_glyph_selection fails with `/Differences` references unresolved; the direct control passes) | test control: indirect values need resolving at consumer

### G014 — font comments cite retired Courier-BoldOblique spelling

- crates: pdf-font
- source files: crates/pdf-font/src/standard.rs?
- test files: none located

- 2038-D43 | pdf-font | source-premise-correction | fixed (round 2074: `standard.rs` and `substitute.rs` name `Courier-BoldOblique` as the clause's, per Issue #384, and the unhyphenated spelling as an alias; the 9.6.2.2 ledger note follows) | font comments cite retired Courier-BoldOblique spelling

### G015 — direct CIDFont descriptor loading

- crates: pdf-font, pdf-model
- source files: crates/pdf-font/src/embed.rs?
- test files: crates/pdf-model/tests/direct_cid_font_descriptor_spec_audit.rs

- 2038-T127 | pdf-font,pdf-model | assertion-strengthening | fixed (round 2074: fails with a direct `/FontDescriptor` ignored in `loading/opening.rs`) | direct CIDFont descriptor loading

### G016 — CMap UseCMap inheritance

- crates: pdf-font
- source files: crates/pdf-font/src/cmap.rs
- test files: crates/pdf-model/tests/encoding_cmap_usecmap_spec_audit.rs

- 2038-T129 | pdf-font | assertion-strengthening | fixed (round 2074: fails with `CMap::parse` given no base) | CMap UseCMap inheritance


## pdf-model

### G017 — ActualText from property resource; ActualText multi-language runs (ADR2812); ActualText per-MCID vs element; Alt/E language escapes not applied; BaselineShift reset at blocks doc; ListNumbering rationale doc; Unicode language escape ignored for property Lang; empty Lang treated as absent; empty value treated as absent; flattened form attributes not reaching accessibility; language precedence test gap; structure traversal bounds truncate silently; structure tree K child reading; table header search stops at conflicting scope; test control: absent attribute classes; test control: hex Lang values; test control: stream-valued structure attributes; writing mode direction test gap

- crates: pdf-model, viewer-accessibility, viewer-core
- source files: crates/pdf-model/src/accessibility.rs, crates/pdf-model/src/content/marked.rs, crates/pdf-model/src/content/run/marked_ops.rs, crates/pdf-model/src/structure.rs, crates/pdf-model/src/structure/attributes.rs, crates/pdf-model/src/structure/logical.rs, crates/viewer-core/src/accessibility.rs
- test files: crates/pdf-model/tests/accessibility.rs, crates/pdf-model/tests/empty_actualtext_spec_audit.rs, crates/pdf-model/tests/missing_attribute_class_spec_audit.rs, crates/pdf-model/tests/named_property_actualtext_spec_audit.rs, crates/pdf-model/tests/structure_attribute_stream_audit.rs, crates/pdf-model/tests/structure_limit_reporting_audit.rs, crates/pdf-model/tests/structure_objr_page_spec_audit.rs, crates/pdf-model/tests/text_strings_spec_audit.rs, crates/pdf-model/tests/writing_mode_edges_spec_audit.rs, crates/viewer-accessibility/tests/tree.rs, crates/viewer-core/tests/headless.rs, crates/viewer-core/tests/lang_hex_consumer_spec_audit.rs, crates/viewer-core/tests/print_field_consumer_spec_audit.rs, crates/viewer-core/tests/structure_phrase_language_spec_audit.rs, crates/viewer-core/tests/table_header_conflicting_scope_spec_audit.rs

- 2038-B01 | pdf-model | behavior-gap | fixed (ADR 2976; text_strings_spec_audit::an_alternate_description_language_escape_overrides_the_property_language, accessibility.rs::a_phrase_changes_language_where_its_escape_stands) | Unicode language escape ignored for property Lang | 1 manifest failure
- 2038-B02 | pdf-model | behavior-gap | fixed (ADR 2976; text_strings_spec_audit::an_expansion_language_escape_overrides_the_property_language) | Unicode language escape ignored for property Lang | 1 manifest failure
- 2038-B03 | pdf-model | behavior-gap | fixed (ADR 2981; structure_limit_reporting_audit::a_large_k_array_is_either_read_whole_or_reported_truncated) | structure traversal bounds truncate silently | 1 manifest failure
- 2038-B04 | pdf-model | behavior-gap | fixed (ADR 2981; structure_limit_reporting_audit::a_deep_hierarchy_is_either_read_whole_or_reported_truncated) | structure traversal bounds truncate silently | 1 manifest failure
- 2038-B12 | pdf-model | behavior-gap | fixed (ADR 2976; headless::structure_actualtext_replaces_two_owned_content_items_once, ::structure_actualtext_replaces_content_enclosed_in_descendant_elements) | ActualText per-MCID vs element | 2 manifest failures
- 2038-B29 | viewer-core,pdf-model | implementation-consumer-gap | fixed (ADR 2978; print_field_consumer_spec_audit, three tests; AccessibilityNode::print_field crosses the confined protocol, panel_answers round trip) | flattened form attributes not reaching accessibility | 3 manifest failures
- 2038-B30 | pdf-model | behavior-and-incorrect-test-oracle | fixed (ADR 2976; accessibility::an_empty_language_identifier_explicitly_states_unknown, ::an_explicitly_unknown_language_shadows_outer_and_ancestral_languages_then_resumes, accessibility.rs::an_unknown_language_shadows_the_default) | empty Lang treated as absent | 2 manifest failures
- 2038-B31 | viewer-core,pdf-model | implementation-consumer-gap | fixed (ADR 2976; structure_phrase_language_spec_audit, two tests) | Alt/E language escapes not applied | 2 manifest failures
- 2038-B34 | pdf-model | implementation-empty-value-gap | fixed (ADR 2976; empty_actualtext_spec_audit, three tests; empty /Alt and /E stay no phrase, not adjudicated by the finding) | empty value treated as absent | 3 manifest failures
- 2038-B53 | viewer-core,pdf-model | adopted-algorithm-recommendation-gap | fixed (ADR 2977, EC3 #200; table_header_conflicting_scope_spec_audit::conflicting_scope_with_headers_does_not_terminate_the_adopted_column_search) | table header search stops at conflicting scope | 1 manifest failure
- 2038-T03 | pdf-model | stale-decision-residual | fixed (ADR 2976 supersedes ADR 2812 §2's last sentence; text_strings_spec_audit's ActualText runs, accessibility.rs::a_phrase_changes_language_where_its_escape_stands) | ActualText multi-language runs (ADR2812)
- 2038-T04 | pdf-model | test-coverage-gap | fixed (the audit's structure_attribute_stream_audit passes; nothing remaining) | test control: stream-valued structure attributes
- 2038-T10 | pdf-model,viewer-core | test-coverage-gap | fixed (the audit's tests pass; the cross-item case is B12's, now held by headless's two tests) | ActualText per-MCID vs element
- 2038-T43 | pdf-model,viewer-core | public-integration-and-fixture-gap | fixed (the audit's fixtures pass; accessibility::an_explicitly_unknown_language_shadows_outer_and_ancestral_languages_then_resumes adds the empty-identifier case) | language precedence test gap
- 2038-T51 | viewer-accessibility,viewer-core | assertion-coverage-gap | fixed (the audit's writing-mode tests pass; nothing remaining) | writing mode direction test gap
- 2038-T80 | pdf-model | test-coverage-gap | fixed (missing_attribute_class_spec_audit passes; nothing remaining) | test control: absent attribute classes
- 2038-T93 | viewer-core | test-coverage-gap | fixed (the audit's hex /Lang tests pass; nothing remaining) | test control: hex Lang values
- 2038-T155 | pdf-model | assertion-quality | fixed (named_property_actualtext_spec_audit passes; rustfmt applied to it, A868/A875) | ActualText from property resource
- 2038-T163 | pdf-model | assertion-quality | fixed (the audit's K-child test passes; nothing remaining) | structure tree K child reading
- 2038-T166 | pdf-model | assertion-quality | fixed (structure_objr_page tests pass; nothing remaining) | structure tree K child reading
- 2038-T172 | pdf-model | assertion-quality | fixed (the repaired fixtures pass; nothing remaining) | structure tree K child reading
- 2038-D17 | viewer-accessibility,pdf-model | source-interpretation-residual | fixed (pdf-model structure/tables.rs: ListNumbering's classification named as a fact this reader does not carry) | ListNumbering rationale doc
- 2038-D33 | viewer-accessibility,pdf-model | representation-policy-boundary | fixed (viewer-core accessibility.rs `Inline`: the block reset is a mapping choice, Table 377's applicability stated apart; ADR 2384 kept) | BaselineShift reset at blocks doc

### G018 — FDF script callback order; choice selection keyed by display label; live field property not reaching later edits

- crates: pdf-model, viewer-core
- source files: crates/pdf-model/src/view/fields.rs, crates/pdf-model/src/view/script_model.rs
- test files: crates/pdf-model/tests/live_field_flags_spec_audit.rs, crates/pdf-model/tests/max_len_live_property_spec_audit.rs, crates/viewer-core/tests/duplicate_choice_labels_spec_audit.rs, crates/viewer-core/tests/fdf_script_order_audit.rs

- 2038-B26 | pdf-model,viewer-core | behavior-defect | fixed (ADR 2979; duplicate_choice_labels_spec_audit::duplicate_labels_keep_both_array_selected_indices) | choice selection keyed by display label | 1 manifest failure
- 2038-B67 | pdf-model | behavior-gap | fixed (ADR 2980; max_len_live_property_spec_audit, four tests) | live field property not reaching later edits | 4 manifest failures
- 2038-B69 | pdf-model | behavior-gap | fixed (ADR 2980; live_field_flags_spec_audit, three tests) | live field property not reaching later edits | 3 manifest failures
- 2038-T02 | viewer-core,pdf-model | test-order-gap | fixed (fdf_script_order_audit passes; usage-rights readiness stays unobservable, ADR 2816 §5 unchanged) | FDF script callback order

### G019 — field attribute read up chain though not inheritable

- crates: pdf-model
- source files: crates/pdf-model/src/form.rs, crates/pdf-model/src/inheritance.rs, crates/pdf-model/src/rich_text.rs
- test files: crates/pdf-model/tests/choice_locality_spec_audit.rs, crates/pdf-model/tests/rich_text_default_style_noninheritance_spec_audit.rs

- 2038-B63 | pdf-model | behavior-gap | fixed (ADR 2979; rich_text_default_style_noninheritance_spec_audit) | field attribute read up chain though not inheritable | 1 manifest failure
- 2038-B64 | pdf-model | behavior-gap | fixed (ADR 2979; choice_locality_spec_audit, two tests) | field attribute read up chain though not inheritable | 2 manifest failures
- 2038-B65 | pdf-model | qualified-behavior-gap | fixed (ADR 2979; choice_locality_spec_audit::choice_opt_stays_with_the_dictionary_that_states_it) | field attribute read up chain though not inheritable | 1 manifest failure

### G020 — disjoint page boundary falls back to larger box; hard depth cap silently drops results; test control: destination XYZ geometry; test oracle: page boundary defaults

- crates: pdf-model, pdf-syntax, viewer-core
- source files: crates/pdf-model/src/page.rs
- test files: crates/pdf-model/tests/deep_page_tree_order_spec_audit.rs, crates/pdf-model/tests/named_page_geometry_spec_audit.rs, crates/pdf-model/tests/page_geometry.rs

- 2038-B19 | pdf-model | behavior-decision-mismatch | fixed (ADR 2981; page_geometry::a_disjoint_page_boundary_clips_all_content_instead_of_defaulting_to_the_medium) | disjoint page boundary falls back to larger box | 1 manifest failure
- 2038-B48 | pdf-model | page-order-after-depth-limit | fixed (ADR 2981; deep_page_tree_order_spec_audit::valid_depth_65_tree_keeps_kids_order_and_corresponding_page_content, deep_page_tree.rs, three tests) | hard depth cap silently drops results | 1 manifest failure
- 2038-T14 | pdf-model | test-oracle-and-coverage-gap | fixed (page_geometry's literal-oracle tests pass; the policy controls are as the finding leaves them) | test oracle: page boundary defaults
- 2038-T70 | pdf-model,viewer-core | test-coverage-gap | fixed (the audit's XYZ geometry controls pass; scroll-bound policy not adjudicated) | test control: destination XYZ geometry
- 2038-D41 | pdf-model,pdf-syntax | tree-limit-contract-and-comment | fixed (ADR 2981; pdf-syntax tree.rs MAX_DEPTH comment and page.rs MAX_TREE_DEPTH comment) | hard depth cap silently drops results

### G021 — FreeText annotation fixtures; pointer-driven annotation appearance state; test control: BS vs Border precedence; test control: NoZoom/NoRotate extents; test control: annotation BM; test control: annotation flag print/hover policy; test control: indirect values need resolving at consumer; test control: widget AS versus inherited V

- crates: pdf-model, viewer-core
- source files: crates/pdf-model/src/annotation.rs, crates/viewer-core/src/viewer.rs
- test files: crates/pdf-model/tests/annotation_border_audit.rs, crates/pdf-model/tests/annotation_output_flags_spec_audit.rs, crates/pdf-model/tests/annotation_view_matrix_spec_audit.rs, crates/pdf-model/tests/appearance_state_precedence_spec_audit.rs, crates/pdf-model/tests/freetext_cloud_border_spec_audit.rs?, crates/pdf-model/tests/indirect_annotation_values_spec_audit.rs, crates/pdf-model/tests/stored_annotation_blend_spec_audit.rs, crates/viewer-core/tests/unknown_annotation_pointer_spec_audit.rs

- 2038-B68 | viewer-core,pdf-model | behavior-gap | fixed (ADR 2982; unknown_annotation_pointer_spec_audit) | pointer-driven annotation appearance state | 1 manifest failure
- 2038-T25 | pdf-model | test-coverage-gap | fixed (audit test annotation_border_audit passes; nothing remaining) | test control: BS vs Border precedence
- 2038-T26 | pdf-model | test-coverage-gap | fixed (audit test stored_annotation_blend_spec_audit passes; nothing remaining) | test control: annotation BM
- 2038-T68 | pdf-model | test-coverage-gap | fixed (audit test appearance_state_precedence_spec_audit passes; nothing remaining) | test control: widget AS versus inherited V
- 2038-T75 | pdf-model | test-coverage-gap | fixed (audit test indirect_annotation_values_spec_audit passes; nothing remaining) | test control: indirect values need resolving at consumer
- 2038-T86 | pdf-model | test-coverage-gap | fixed (audit test annotation_view_matrix_spec_audit passes; nothing remaining) | test control: NoZoom/NoRotate extents
- 2038-T87 | pdf-model | test-coverage-gap | fixed (audit test annotation_output_flags_spec_audit passes; nothing remaining) | test control: annotation flag print/hover policy
- 2038-T140 | pdf-model | fixture-validity-strengthening | fixed (audit fixtures in the FreeText callout tests pass; nothing remaining) | FreeText annotation fixtures
- 2038-T143 | pdf-model | fixture-validity | fixed (audit fixtures in the FreeText tests pass; nothing remaining) | FreeText annotation fixtures

### G022 — FDF Encoding utf_8 ignored; FDF fixture noncanonical header; doc: FormsData Pages conflict comment; test control: FDF templates; test control: NeedAppearances save

- crates: doc, pdf-model
- source files: crates/pdf-model/src/forms_data.rs
- test files: crates/pdf-model/tests/fdf_template_file_spec_audit.rs, crates/pdf-model/tests/fdf_template_pixels_spec_audit.rs, crates/pdf-model/tests/fdf_utf8_import_spec_audit.rs, crates/pdf-model/tests/need_appearances_all_visible_widgets_spec_audit.rs?

- 2038-B75 | pdf-model | behavior-gap | fixed (ADR 2983; fdf_utf8_import_spec_audit x3, forms_data::tests::utf_16_and_utf_8_read_unmarked_strings_in_their_own_forms) | FDF Encoding utf_8 ignored | 3 manifest failures
- 2038-T15 | pdf-model | test-coverage-gap | fixed (audit tests fdf_template_pixels_spec_audit, fdf_template_file_spec_audit pass; Persistence and Rename=true stay documented departures) | test control: FDF templates
- 2038-T59 | pdf-model | fixture-evidence-boundary | fixed (audit fixture is %FDF-1.2 with catalog /Version /1.4; passes) | FDF fixture noncanonical header
- 2038-T117 | pdf-model | assertion-strengthening | fixed (audit test need_appearances_all_visible_widgets_spec_audit passes; nothing remaining) | test control: NeedAppearances save
- 2038-D07 | doc,pdf-model | stale-diagnostic-description | fixed (ADR 2983: /Status beside /Pages is now named, so the comment is true; forms_data::tests::status_beside_pages_is_named) | doc: FormsData Pages conflict comment

### G023 — DPart fixtures noncanonical; DParts inventory comments stale; hard depth cap silently drops results

- crates: pdf-model, viewer-core
- source files: crates/pdf-model/src/document_part.rs
- test files: crates/viewer-core/tests/dpart_depth_spec_audit.rs

- 2038-B36 | pdf-model,viewer-core | incomplete-result-disclosure-gap | fixed (ADR 2984: no depth bound, cycles finite by the reference set; dpart_depth_spec_audit, document_part::tests::a_deep_chain_is_read_whole) | hard depth cap silently drops results | 1 manifest failure
- 2038-T48 | pdf-model,viewer-core | fixture-and-route-evidence-gap | fixed (audit current-shape DPart and annotation click fixtures pass) | DPart fixtures noncanonical
- 2038-D21 | pdf-model | source-errata-and-rationale-gap | fixed (ADR 2984: comments name Issue #612 flat /DParts as normative, nested arrays as the retired shape; doc/errata-read.md row untouched, a record) | DParts inventory comments stale
- 2038-D25 | pdf-model | resource-limit-rationale-gap | fixed (ADR 2984: the depth bound and its cycle claim are gone) | hard depth cap silently drops results

### G024 — named destination lookup order; named destination name tree lookup; test control: indirect name and number trees

- crates: pdf-model
- source files: crates/pdf-model/src/destination.rs
- test files: crates/pdf-model/tests/deep_name_tree_spec_audit.rs, crates/pdf-model/tests/deep_number_tree_spec_audit.rs, crates/viewer-core/tests/named_destination_tables_audit.rs

- 2038-B08 | pdf-model | behavior-interpretation-mismatch | fixed (ADR 2985 supersedes ADR 0054 on lookup order; named_destination_tables_audit) | named destination lookup order | 1 manifest failure
- 2038-T23 | pdf-model | test-coverage-gap | fixed (audit tests deep_name_tree_spec_audit, deep_number_tree_spec_audit pass) | test control: indirect name and number trees
- 2038-T149 | pdf-model | assertion-quality | fixed (strengthened named-destination and page-label tests pass) | named destination name tree lookup

### G025 — hard cap silently reports absent; reference ID-only matching policy doc; reference XObject print test gap

- crates: pdf-model, viewer-core
- source files: crates/pdf-model/src/reference.rs
- test files: crates/pdf-model/tests/reference_page_label_limit_spec_audit.rs, crates/viewer-core/tests/reference_print_spec_audit.rs

- 2038-B32 | pdf-model | limit-misreported-as-absence | fixed (§12.4.2 inverted per range: PageLabels::index_of, no page walk and no bound; reference_page_label_limit_spec_audit, page_label::tests::index_of_inverts_label_on_every_page) | hard cap silently reports absent | 1 manifest failure
- 2038-T45 | viewer-core,pdf-model | public-integration-test-gap | fixed (audit test reference_print_spec_audit passes; pins a permitted product choice) | reference XObject print test gap
- 2038-D19 | pdf-model | source-policy-boundary | fixed (reference.rs module doc: identifier-only matching is this project's policy (ADR 1101), the match sentence is §14.4's, first supplied candidate taken) | reference ID-only matching policy doc

### G026 — collection folder tree rows; collection sort direction array

- crates: pdf-model
- source files: crates/pdf-model/src/collection.rs
- test files: crates/viewer-host/tests/collection_folder_tree_spec_audit.rs, crates/viewer-host/tests/collection_sort_date_spec_audit.rs

- 2038-B76 | pdf-model | behavior-gap | fixed (Table 156: a boolean /A is stored once per /S field, an array's missing entries default true; collection_sort_date_spec_audit, collection::tests::a_boolean_directs_every_field_and_a_short_array_only_its_own) | collection sort direction array | 1 manifest failure
- 2038-T164 | pdf-model | assertion-quality | fixed (audit control test passes; its B76 witness now passes) | collection sort direction array
- 2038-T167 | pdf-model | assertion-quality | fixed (audit test collection_folder_tree_spec_audit passes) | collection folder tree rows

### G027 — tiling pattern lattice spacing

- crates: pdf-model, pdf-render
- source files: crates/pdf-model/src/content/pattern/lattice.rs, crates/pdf-render/src/lattice.rs
- test files: crates/pdf-model/tests/tiling.rs, crates/pdf-model/tests/tiling_signed_steps_spec_audit.rs

- 2038-B72 | pdf-model,pdf-render | behavior-gap | fixed (ADR 2987 amends ADR 1080: pdf_render::snap_steps where the matrix snap is refused; tiling::type_one_keeps_constant_pixel_spacing..., lattice::tests::a_step_on_a_device_axis_snaps_to_whole_pixels) | tiling pattern lattice spacing | 1 manifest failure
- 2038-T133 | pdf-model,pdf-render | assertion-strengthening | fixed (audit test tiling_signed_steps_spec_audit passes) | tiling pattern lattice spacing

### G028 — page visibility script triggers; test control: indirect values need resolving at consumer

- crates: pdf-model, viewer-core
- source files: crates/pdf-model/src/view/script_sites.rs, crates/viewer-core/src/viewer/navigation.rs
- test files: crates/viewer-core/tests/catalog_aa_indirect_reference_spec_audit.rs, crates/viewer-core/tests/page_visible_script_runner_spec_audit.rs

- 2038-B78 | viewer-core,pdf-model | behavior-gap | fixed (ADR 2986 supersedes ADR 2814 section 3; page_visible_script_runner_spec_audit x3) | page visibility script triggers | 3 manifest failures
- 2038-T116 | pdf-model,viewer-core | assertion-strengthening | fixed (audit test catalog_aa_indirect_reference_spec_audit passes) | test control: indirect values need resolving at consumer

### G029 — JP2 opacity channel counted as colour

- crates: pdf-model
- source files: crates/pdf-model/src/image/jpx.rs?
- test files: crates/pdf-model/tests/jpx_ordinary_opacity_audit.rs

- 2038-B14 | pdf-model | behavior-gap | fixed (ISO/IEC 15444-1 I.5.3.6 Table I-16 kind 2: jpx::premultiplied_opacity_channel; jpx_ordinary_opacity_audit) | JP2 opacity channel counted as colour | 1 manifest failure

### G030 — degenerate tiling BBox dropped

- crates: pdf-model, render-raster
- source files: crates/pdf-model/src/content/pattern/tiling.rs
- test files: crates/pdf-model/tests/degenerate_tiling_bbox_clip_spec_audit.rs

- 2038-B20 | pdf-model,render-raster | behavior-and-adr-mismatch | open (Table 74 NOTE 1 and §10.7.4: a zero-height /BBox must clip and still scan-convert one pixel; pdf-model keeps it unclipped because a zero-area clip removes everything on both backends. The fix is a degenerate-clip rule in render-cpu and render-raster, outside this group's pdf-model file; ADR 1080/tiling.rs comment unchanged) | degenerate tiling BBox dropped | 1 manifest failure

### G031 — page output intent replaced by catalog

- crates: pdf-model
- source files: crates/pdf-model/src/output_intent.rs
- test files: crates/pdf-model/tests/output_intent_page_scope_spec_audit.rs

- 2038-B33 | pdf-model | implementation-scope-gap | fixed (§14.11.5 page-level precedence with Table 401's optional profile: output_intent::selected and in_force_on_pages no longer borrow the catalog's intent; output_intent_page_scope_spec_audit x2) | page output intent replaced by catalog | 2 manifest failures

### G032 — non-inheritable field keys read via parents

- crates: pdf-model
- source files: crates/pdf-model/src/aform/site.rs
- test files: crates/pdf-model/tests/field_aa_noninheritance_spec_audit.rs

- 2038-B62 | pdf-model | behavior-gap | fixed (Table 226 does not make /AA inheritable: aform::site::terminal_action reads the widget and its terminal field only; field_aa_noninheritance_spec_audit. Inherited::action removed from inheritance.rs, G019's file, as dead) | non-inheritable field keys read via parents | 1 manifest failure

### G033 — annex I version warning timing doc; classic conversion notes stale; general syntax note stale; interchange prepress notes stale; separation simulation ledger note stale; shading AntiAlias flag unread

- crates: pdf-model, pdf-transform, render-cpu, viewer-core
- source files: doc/ledger-and-claims.md?
- test files: crates/pdf-transform/tests/annex_i_insert_version.rs?

- 2038-D15 | render-cpu | source-interpretation-residual | owner (Q899; the 8.7.4.3 note now says the reading of a true `/AntiAlias` is open) | shading AntiAlias flag unread
- 2038-D22 | viewer-core,pdf-transform | ledger-evidence-wording-gap | fixed (I.2's note: the warning comes in the report hosts ask for after the first frame, ADR 1044; viewer-core notes.rs module doc corrected) | annex I version warning timing doc
- 2038-D30 | pdf-model | contradictory-ledger-rationale | fixed (10.8.3's note says what is built; the partial-status reasoning and the row's history removed) | separation simulation ledger note stale
- 2038-D34 | pdf-model | stale-ledger-rationale | fixed (7.1's note no longer says §7.11 and §7.12 are unread) | general syntax note stale
- 2038-D38 | pdf-model | aggregate-rationale-drift | fixed (10.4 and 10.4.2 notes name ADR 1207's explicit BG/UCR branch, which supersedes ADR 1069) | classic conversion notes stale
- 2038-D39 | pdf-model | stale-aggregate-scope | fixed (14.1 and 14.11 notes match their children's statuses) | interchange prepress notes stale

### G034 — glyph clip pixel test gap; test control: composite font literal escapes; test control: multibyte word spacing, vertical TJ; test oracle too weak or vacuous; text leading and spacing persistence; text stream decoding test gap

- crates: pdf-model
- source files: crates/pdf-model/src/content/text.rs
- test files: crates/pdf-model/tests/composite_literal_escape_spec_audit.rs, crates/pdf-model/tests/text_clip_pixels_spec_audit.rs, crates/pdf-model/tests/text_positioning_audit.rs, crates/pdf-model/tests/text_state.rs, crates/pdf-model/tests/text_state_vertical_spec_audit.rs, crates/pdf-model/tests/text_strings_spec_audit.rs

- 2038-T22 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: text_state_vertical_spec_audit (word spacing on code length; vertical TJ) fails on a plant of the defect and passes restored) | test control: multibyte word spacing, vertical TJ
- 2038-T56 | pdf-model | assertion-coverage-gap | fixed (plant-checked in round 2077: text_strings_spec_audit filtered UTF-8 stream fails on a plant of the defect and passes restored) | text stream decoding test gap
- 2038-T58 | pdf-model | assertion-coverage-gap | fixed (plant-checked in round 2077: text_clip_pixels_spec_audit fails on a plant of the defect and passes restored) | glyph clip pixel test gap
- 2038-T79 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: composite_literal_escape_spec_audit fails on a plant of the defect and passes restored) | test control: composite font literal escapes
- 2038-T100 | pdf-model | test-oracle-gap | fixed (the audit's zero-TJ plant fails the new delta assertion; text_state::horizontal_scaling_reaches_a_tj_adjustment passes) | test oracle too weak or vacuous
- 2038-T146 | pdf-model | coverage-gap | fixed (plant-checked in round 2077: text_positioning_audit TD leading and double-quote spacing fails on a plant of the defect and passes restored) | text leading and spacing persistence

### G035 — annotation rollover down appearance defaults; appearance fallback comment stale; test control: Stamp and Caret supplied appearance; widget border background colours

- crates: pdf-model
- source files: crates/pdf-model/src/appearance.rs
- test files: crates/pdf-model/tests/appearance_state_precedence_spec_audit.rs, crates/pdf-model/tests/markup_stored_appearance_spec_audit.rs, crates/pdf-model/tests/widget_annotation_spec_audit.rs

- 2038-T34 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: markup_stored_appearance_spec_audit Stamp and Caret fails on a plant of the defect and passes restored) | test control: Stamp and Caret supplied appearance
- 2038-T124 | pdf-model | assertion-strengthening | fixed (plant-checked in round 2077: annotation_appearance_audit explicit rollover fails on a plant of the defect and passes restored) | annotation rollover down appearance defaults
- 2038-T161 | pdf-model | assertion-quality | fixed (plant-checked in round 2077: widget_annotation_spec_audit border and background fails on a plant of the defect and passes restored) | widget border background colours
- 2038-D35 | pdf-model | stale-scope-comment | fixed (appearance.rs: the 3D/RichMedia arm's comment says §12.5.6.25's row takes §13.6.2's and §13.7.2.1's implemented status, ADR 1946) | appearance fallback comment stale

### G036 — Type3 font glyph handling; test control: d1 ExtGState restoration; type3 d1 image test gap; type3 fixture validity

- crates: pdf-model
- source files: crates/pdf-model/src/type3.rs
- test files: crates/pdf-model/tests/type3.rs

- 2038-T12 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: type3_shape_state_restoration fails on a plant of the defect and passes restored) | test control: d1 ExtGState restoration
- 2038-T40 | pdf-model | test-fixture-validity-gap | fixed (the audit corrected the Type 3 fixtures' extents and labelled the named-Encoding tolerance; type3 passes. Its remaining MMType1 instance is a Type 1 coverage gap outside this group) | type3 fixture validity
- 2038-T64 | pdf-model | passing-regression | fixed (plant-checked in round 2077: type3_image_restrictions fails on a plant of the defect and passes restored) | type3 d1 image test gap
- 2038-T131 | pdf-model | assertion-strengthening | fixed (plant-checked in round 2077: type3 missing glyph fails on a plant of the defect and passes restored) | Type3 font glyph handling

### G037 — test control: indirect values need resolving at consumer; test control: mesh shading topology

- crates: pdf-model
- source files: crates/pdf-model/src/shading.rs?
- test files: crates/pdf-model/tests/indirect_shading_values_spec_audit.rs, crates/pdf-model/tests/type4_edge_flags_spec_audit.rs, crates/pdf-model/tests/type5_function_spec_audit.rs

- 2038-T81 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: indirect_shading_values_spec_audit fails on a plant of the defect and passes restored) | test control: indirect values need resolving at consumer
- 2038-T96 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: type4_edge_flags_spec_audit fails on a plant of the defect and passes restored) | test control: mesh shading topology
- 2038-T97 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: shadings lattice rows fails on a plant of the defect and passes restored) | test control: mesh shading topology
- 2038-T98 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: type5_function_spec_audit fails on a plant of the defect and passes restored) | test control: mesh shading topology

### G038 — media software identifier scope doc; rich media fixtures partial; test control: media data BU judge

- crates: pdf-model, viewer-core
- source files: crates/pdf-model/src/media.rs?
- test files: crates/pdf-model/tests/media_viability.rs, crates/viewer-core/tests/rich_media_assets.rs

- 2038-T55 | viewer-core,pdf-model | fixture-evidence-boundary | fixed (the audit's module doc in rich_media_assets.rs states the fixture's consumer-only scope; nothing remaining) | rich media fixtures partial
- 2038-T82 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: media_viability BU (MH half planted) fails on a plant of the defect and passes restored) | test control: media data BU judge
- 2038-D31 | pdf-model | scoped-implementation-boundary | fixed (13.2.7.4 and 13.2.7.4.4 notes say the status certifies the nameless-software decisions only: URI, OS and range comparisons are not built, owed once a host names its player) | media software identifier scope doc

### G039 — compatibility section test gap; contents array q/Q test gap; contents array token split comment

- crates: pdf-model
- source files: crates/pdf-model/src/content/reader.rs?
- test files: crates/pdf-model/tests/compatibility_operator_pixels_spec_audit.rs, crates/pdf-model/tests/contents_q_stack_cross_part_spec_audit.rs

- 2038-T47 | pdf-model | assertion-coverage-gap | fixed (plant-checked in round 2077: compatibility_operator_pixels_spec_audit fails on a plant of the defect and passes restored) | compatibility section test gap
- 2038-T63 | pdf-model | passing-regression | fixed (plant-checked in round 2077: contents_q_stack_cross_part_spec_audit fails on a plant of the defect and passes restored) | contents array q/Q test gap
- 2038-D37 | pdf-model | source-comment-inaccuracy | fixed (page.rs `content`'s doc quotes Table 31: parts divide only at token boundaries, and the newline is the stated white space) | contents array token split comment

### G040 — image Decode defaults; test control: predictor with image bit depth; test oracle too weak or vacuous

- crates: pdf-model
- source files: crates/pdf-model/src/image.rs?
- test files: crates/pdf-model/tests/devicen_decode_defaults_spec_audit.rs, crates/pdf-model/tests/indexed_rounding_initial_spec_audit.rs, crates/pdf-model/tests/predictor_image_bpc_spec_audit.rs

- 2038-T94 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: predictor_image_bpc_spec_audit fails on a plant of the defect and passes restored) | test control: predictor with image bit depth
- 2038-T101 | pdf-model | test-oracle-gap | fixed (the audit's index-minus-one plant fails the 256-index assertion; indexed_rounding_initial_spec_audit passes) | test oracle too weak or vacuous
- 2038-T150 | pdf-model | coverage-gap | fixed (plant-checked in round 2077: devicen_decode_defaults_spec_audit and the inline D test fails on a plant of the defect and passes restored) | image Decode defaults

### G041 — test control: ExtGState Font; test control: UseBlackPtComp state; test control: indirect values need resolving at consumer

- crates: pdf-model
- source files: crates/pdf-model/src/content/graphics_state.rs?
- test files: crates/pdf-model/tests/extgstate_font_valid_consumer_spec_audit.rs, crates/pdf-model/tests/indirect_extgstate_values_spec_audit.rs, crates/pdf-model/tests/rendering_intent.rs?

- 2038-T77 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: indirect_extgstate_values_spec_audit fails on a plant of the defect and passes restored) | test control: indirect values need resolving at consumer
- 2038-T95 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: rendering_intent UseBlackPtComp fails on a plant of the defect and passes restored) | test control: UseBlackPtComp state
- 2038-T115 | pdf-model | assertion-strengthening | fixed (plant-checked in round 2077: extgstate_font_valid_consumer_spec_audit fails on a plant of the defect and passes restored) | test control: ExtGState Font

### G042 — alpha group/transfer pixel controls; soft mask graphics state restoration; soft mask group colour space

- crates: pdf-model
- source files: crates/pdf-model/src/soft_mask.rs
- test files: crates/pdf-model/tests/coordinate_graphics_state_spec_audit.rs, crates/pdf-model/tests/soft_masks.rs

- 2038-T60 | pdf-model | strengthened-public-assertions | fixed (plant-checked in round 2077: soft_masks Alpha transfer and empty group fails on a plant of the defect and passes restored) | alpha group/transfer pixel controls
- 2038-T122 | pdf-model | assertion-strengthening | fixed (plant-checked in round 2077: soft_masks none_removes_the_mask fails on a plant of the defect and passes restored) | soft mask graphics state restoration
- 2038-T135 | pdf-model | assertion-strengthening | fixed (plant-checked in round 2077: soft_masks inherited group space fails on a plant of the defect and passes restored) | soft mask group colour space

### G043 — audit fixture producer validity; embedded file stream selection; test control: associated files in content

- crates: pdf-model
- source files: crates/pdf-model/src/attachment.rs
- test files: crates/pdf-model/tests/associated_file_examples_audit.rs, crates/pdf-model/tests/collections.rs?, crates/pdf-model/tests/uf_embedded_stream_selection_spec_audit.rs

- 2038-T19 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: nested_mcaf_spec_audit form, Type 3, tiling fails on a plant of the defect and passes restored) | test control: associated files in content
- 2038-T132 | pdf-model | assertion-strengthening | fixed (plant-checked in round 2077: uf_embedded_stream_selection_spec_audit fails on a plant of the defect and passes restored) | embedded file stream selection
- 2038-T147 | pdf-model | fixture-validity | fixed (the audit's fixture repairs stand; the four collection and two image failures it kept are other findings' manifest entries) | audit fixture producer validity

### G044 — shading pattern matrix and stream; type1 shading public raster test gap

- crates: pdf-model, render-raster
- source files: crates/pdf-model/src/content/pattern/shading.rs
- test files: crates/pdf-model/tests/shading_pattern_matrix_spec_audit.rs, crates/pdf-model/tests/shading_pattern_stream_spec_audit.rs, crates/pdf-model/tests/shadings.rs

- 2038-T39 | pdf-model,render-raster | public-integration-test-gap | fixed (plant-checked in round 2077: shadings Type 1 matrix/domain (CPU half) fails on a plant of the defect and passes restored) | type1 shading public raster test gap
- 2038-T165 | pdf-model | assertion-quality | fixed (plant-checked in round 2077: shading_pattern_matrix_spec_audit fails on a plant of the defect and passes restored) | shading pattern matrix and stream
- 2038-T169 | pdf-model | assertion-quality | fixed (plant-checked in round 2077: shading_pattern_stream_spec_audit fails on a plant of the defect and passes restored) | shading pattern matrix and stream

### G045 — grid-fault API without viewer call site; test control: artifact reporting

- crates: doc, pdf-model, viewer-core
- source files: crates/pdf-model/src/content/report.rs, crates/viewer-core/src/report.rs?
- test files: crates/pdf-model/tests/artifact_reporting_audit.rs, crates/pdf-model/tests/function_faults.rs

- 2038-T07 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: artifact_reporting_audit fails on a plant of the defect and passes restored) | test control: artifact reporting
- 2038-D05 | doc,pdf-model,viewer-core | acknowledged-integration-limit | fixed (premise no longer holds: ADR 2946's `said_after_drawing` in viewer-core/src/viewer/opening.rs is the viewer call site of `Interpretation::after_drawing`; viewer-core tests/function_faults_after_drawing.rs; GPU readback is ADR 2940's) | grid-fault API without viewer call site

### G046 — test oracle independent of production helper

- crates: pdf-colour, pdf-model
- source files: crates/pdf-colour/src/lib.rs?
- test files: crates/pdf-model/tests/calrgb_literal_samples_spec_audit.rs, crates/pdf-model/tests/spot_press.rs?

- 2038-T85 | pdf-model,pdf-colour | test-oracle-improvement | fixed (the audit's zero-transfer plant fails the CalRGB literal pixels; calrgb_literal_samples_spec_audit passes) | test oracle independent of production helper
- 2038-T102 | pdf-model | test-oracle-gap | fixed (plant-checked in round 2077: transparency_groups three press routes, literal pixels fails on a plant of the defect and passes restored) | test oracle independent of production helper

### G047 — fractional alpha compositing test gap; test control: transparency group reuse

- crates: pdf-model, render-raster
- source files: crates/pdf-model/src/content/transparency.rs?
- test files: crates/pdf-model/tests/transparency_groups.rs, crates/pdf-model/tests/transparency_repeated_group_spec_audit.rs

- 2038-T44 | pdf-model,render-raster | independent-numeric-oracle-gap | fixed (plant-checked in round 2077: render-cpu basic_composite_partial_alpha_spec_audit (Color mode planted) fails on a plant of the defect and passes restored) | fractional alpha compositing test gap
- 2038-T105 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: transparency_groups and transparency_repeated_group_spec_audit fails on a plant of the defect and passes restored) | test control: transparency group reuse

### G048 — OC language precedence test gap; optional content recommendation AND

- crates: pdf-model
- source files: crates/pdf-model/src/optional_content.rs
- test files: crates/pdf-model/tests/optional_content.rs, crates/pdf-model/tests/optional_content_nested_spec_audit.rs?

- 2038-T66 | pdf-model | test-coverage-gap | fixed (plant-checked in round 2077: optional_content exact-match language (partial-tie half not planted) fails on a plant of the defect and passes restored) | OC language precedence test gap
- 2038-T123 | pdf-model | assertion-strengthening | fixed (the audit's last-writer-wins plant fails the AND assertion in optional_content.rs, which passes) | optional content recommendation AND

### G049 — embedded file checksum test gap; embedded file description reading

- crates: pdf-model
- source files: crates/pdf-model/src/file_spec.rs
- test files: crates/pdf-model/tests/embedded_files_spec_audit.rs?, crates/pdf-model/tests/embedded_flate_checksum_spec_audit.rs

- 2038-T62 | pdf-model | passing-regression | fixed (plant-checked in round 2077: embedded_flate_checksum_spec_audit fails on a plant of the defect and passes restored) | embedded file checksum test gap
- 2038-T148 | pdf-model | fixture-validity-and-assertion-quality | fixed (the audit's six fixture repairs and description assertions stand; its four failures are other findings' manifest entries) | embedded file description reading

### G050 — cmyk image overprint test gap; group overprint OPM

- crates: pdf-model
- source files: crates/pdf-model/src/content/overprint.rs
- test files: crates/pdf-model/tests/cmyk_image_overprint_spec_audit.rs, crates/pdf-model/tests/overprint_group_row_spec_audit.rs

- 2038-T57 | pdf-model | assertion-coverage-gap | fixed (plant-checked in round 2077: cmyk_image_overprint_spec_audit fails on a plant of the defect and passes restored) | cmyk image overprint test gap
- 2038-T171 | pdf-model | assertion-quality | fixed (plant-checked in round 2077: overprint_group_row_spec_audit fails on a plant of the defect and passes restored) | group overprint OPM

### G051 — EC3 enum19 JPX doc

- crates: pdf-model
- source files: crates/pdf-model/src/jpeg2000.rs?
- test files: crates/pdf-model/tests/jpx_ordinary_opacity_audit.rs?

- 2038-D20 | pdf-model | effective-source-and-test-scope | open | EC3 enum19 JPX doc

### G052 — degenerate stroke caps composited separately

- crates: pdf-model, render-cpu
- source files: doc/adr/0420-the-mark-a-placement-took-after-the-alpha-survived.md
- test files: none located

- 2038-D29 | pdf-model,render-cpu | decision-counterexample | open | degenerate stroke caps composited separately

### G053 — test gates pass after skipping absent corpus file

- crates: pdf-model
- source files: none (test or document only)
- test files: crates/pdf-model/tests/fixed_documents.rs?

- 2038-D45 | pdf-model | assertion-execution-gap | open | test gates pass after skipping absent corpus file

### G054 — fixed-document gate stale font expectations

- crates: pdf-model
- source files: none (test or document only)
- test files: crates/pdf-model/tests/fixed_documents.rs

- 2038-D51 | pdf-model | stale-test-expectation | open | fixed-document gate stale font expectations | 1 manifest failure; passes on this VM only because its row is skipped (corpus batch5 absent, Q882)

### G055 — test control: JBIG2 filter chain

- crates: pdf-model
- source files: crates/pdf-model/src/image/jbig2.rs?
- test files: crates/pdf-model/tests/jbig2_filter_chain_audit.rs

- 2038-T05 | pdf-model | test-coverage-gap | open | test control: JBIG2 filter chain

### G056 — test control: CCITT EndOfBlock

- crates: pdf-ccitt, pdf-model
- source files: crates/pdf-model/src/image/ccitt.rs?
- test files: crates/pdf-model/tests/ccitt_eob_rows_spec_audit.rs

- 2038-T06 | pdf-model,pdf-ccitt | test-claim-qualification | open | test control: CCITT EndOfBlock

### G057 — test control: BG/UCR transfer

- crates: pdf-model
- source files: crates/pdf-model/src/image/convert.rs?
- test files: crates/pdf-model/tests/black_generation_transfer_audit.rs

- 2038-T08 | pdf-model | test-coverage-gap | open | test control: BG/UCR transfer

### G058 — test fixture misstated stream lengths

- crates: pdf-model
- source files: none (test or document only)
- test files: crates/pdf-model/tests/contents_entry.rs, crates/pdf-model/tests/damaged_page_dictionaries.rs

- 2038-T103 | pdf-model | fixture-isolation-gap | open | test fixture misstated stream lengths

### G059 — test fixture ICC header not conforming

- crates: pdf-model
- source files: none (test or document only)
- test files: crates/pdf-model/tests/rendering_intent.rs?

- 2038-T107 | pdf-model | fixture-validity-gap | open | test fixture ICC header not conforming

### G060 — test control: unreadable font program fallback

- crates: pdf-model
- source files: crates/pdf-font/src/loading.rs?
- test files: crates/pdf-model/tests/unreadable_program_fallback_control_audit.rs

- 2038-T11 | pdf-model | test-coverage-gap | open | test control: unreadable font program fallback

### G061 — test control: named page geometry

- crates: pdf-model, viewer-core
- source files: crates/pdf-model/src/named_pages.rs?
- test files: crates/pdf-model/tests/named_page_geometry_spec_audit.rs?

- 2038-T113 | pdf-model,viewer-core | test-assurance-gap | open | test control: named page geometry

### G062 — field Q quadding precedence

- crates: pdf-model
- source files: crates/pdf-model/src/variable_text.rs
- test files: crates/pdf-model/tests/quadding_field_tree_precedence_spec_audit.rs

- 2038-T120 | pdf-model | assertion-strengthening | open | field Q quadding precedence

### G063 — page label missing style

- crates: pdf-model
- source files: crates/pdf-model/src/page_label.rs
- test files: crates/pdf-model/tests/page_label_missing_style_spec_audit.rs

- 2038-T125 | pdf-model | assertion-strengthening | open | page label missing style

### G064 — FreeText cloud and Link border defaults

- crates: pdf-model
- source files: crates/pdf-model/src/cloud.rs
- test files: crates/pdf-model/tests/freetext_cloud_border_spec_audit.rs

- 2038-T136 | pdf-model | assertion-strengthening | open | FreeText cloud and Link border defaults

### G065 — popup parent metadata

- crates: pdf-model
- source files: crates/pdf-model/src/popup.rs
- test files: crates/viewer-core/tests/text_markup_popup_spec_audit.rs

- 2038-T142 | pdf-model | assertion-strengthening | open | popup parent metadata

### G066 — trailer Info metadata

- crates: pdf-model
- source files: crates/pdf-model/src/metadata.rs
- test files: crates/pdf-model/tests/direct_info_spec_audit.rs

- 2038-T144 | pdf-model | coverage-gap | open | trailer Info metadata

### G067 — DCT image decoding

- crates: pdf-model
- source files: crates/pdf-model/src/image/dct.rs
- test files: crates/pdf-model/tests/colour_transform.rs?

- 2038-T156 | pdf-model | assertion-quality | open | DCT image decoding

### G068 — compatibility section nesting

- crates: pdf-model
- source files: crates/pdf-model/src/content/run.rs?
- test files: crates/pdf-model/tests/compatibility_operator_pixels_spec_audit.rs

- 2038-T160 | pdf-model | assertion-quality | open | compatibility section nesting

### G069 — test control: CIDToGIDMap

- crates: pdf-font, pdf-model
- source files: crates/pdf-font/src/composite.rs?
- test files: crates/pdf-model/tests/cid_map_stream_spec_audit.rs

- 2038-T17 | pdf-model,pdf-font | test-coverage-gap | open | test control: CIDToGIDMap

### G070 — soft mask Matte decode

- crates: pdf-model
- source files: crates/pdf-model/src/image/soft_mask.rs
- test files: crates/pdf-model/tests/matte_after_decode_spec_audit.rs

- 2038-T170 | pdf-model | assertion-quality | open | soft mask Matte decode

### G071 — test oracle: viewer preference defaults

- crates: pdf-model
- source files: crates/pdf-model/src/viewer_preferences.rs?
- test files: crates/pdf-model/tests/viewer_preference_defaults_spec_audit.rs

- 2038-T24 | pdf-model | weak-test-oracle | open | test oracle: viewer preference defaults

### G072 — test control: DeviceRGB reset

- crates: pdf-model
- source files: none (test or document only)
- test files: crates/pdf-model/tests/device_rgb_reset_spec_audit.rs

- 2038-T27 | pdf-model | test-coverage-gap | open | test control: DeviceRGB reset

### G073 — test control: cm composition

- crates: pdf-model
- source files: none (test or document only)
- test files: crates/pdf-model/tests/coordinate_graphics_state_spec_audit.rs

- 2038-T32 | pdf-model | test-coverage-gap | open | test control: cm composition

### G074 — test assertion: robustness q/Q in path

- crates: pdf-model
- source files: none (test or document only)
- test files: crates/pdf-model/tests/graphics_stack_paths_audit.rs

- 2038-T33 | pdf-model | assertion-scope-correction | open | test assertion: robustness q/Q in path

### G075 — test weakness: halftone transfer override

- crates: pdf-model
- source files: crates/pdf-model/src/content/ext_gstate.rs?
- test files: crates/pdf-model/tests/halftone_transfer_public_spec_audit.rs

- 2038-T35 | pdf-model | public-consumer-test-gap | open | test weakness: halftone transfer override

### G076 — model-only routes lack consumer tests

- crates: pdf-model, viewer-core
- source files: crates/pdf-model/src/annotation_state.rs?
- test files: crates/viewer-core/tests/headless.rs

- 2038-T46 | pdf-model,viewer-core | public-integration-test-gap | open | model-only routes lack consumer tests

### G077 — test control: indirect values need resolving at consumer

- crates: pdf-model
- source files: crates/pdf-model/src/attachments.rs?
- test files: crates/pdf-model/tests/indirect_file_parameters_spec_audit.rs

- 2038-T88 | pdf-model | test-coverage-gap | open | test control: indirect values need resolving at consumer

### G078 — test control: inline image filter chain

- crates: pdf-model
- source files: crates/pdf-model/src/content/inline_image.rs?
- test files: crates/pdf-model/tests/inline_filter_chain_spec_audit.rs

- 2038-T92 | pdf-model | test-coverage-gap | open | test control: inline image filter chain


## pdf-signature

### G079 — DocMDP timestamp-only revision unclassified; field rename after signing misclassified; field reset overwrites shared ancestor V; incremental signed writer test gap; signed-field lock fixtures invalid; usage rights retained past granted rights

- crates: pdf-model, pdf-signature, viewer-core
- source files: crates/pdf-model/src/view/writing.rs, crates/pdf-signature/src/revision.rs
- test files: crates/pdf-model/tests/forms_data.rs, crates/pdf-model/tests/inherited_reset_defaults_audit.rs, crates/pdf-model/tests/signature_field_lock_spec_audit.rs, crates/pdf-model/tests/usage_rights_annotation_save_spec_audit.rs, crates/pdf-signature/src/revision.rs?, crates/pdf-signature/tests/docmdp_dts_carveout.rs, crates/viewer-core/tests/headless.rs?

- 2038-B13 | pdf-model | behavior-gap | fixed (ADR 2995; `writing::holder` stops at the terminal field's `/T`; inherited_reset_defaults_audit passes) | field reset overwrites shared ancestor V | 1 manifest failure
- 2038-B18 | pdf-signature | behavior-gap | fixed (ADR 2996; a timestamp's field, widget, `/Fields`, `/Annots` and `/SigFlags` registration is validation material; docmdp_dts_carveout passes, its page-change control still refused) | DocMDP timestamp-only revision unclassified | 1 manifest failure
- 2038-B40 | pdf-model,pdf-signature | adopted-policy-regression | fixed (ADR 2995, amends ADR 0159; a save asks `/Annots /Create` and `/Modify` for what it writes; usage_rights_annotation_save_spec_audit passes) | usage rights retained past granted rights | 1 manifest failure
- 2038-B42 | pdf-signature | unresolved-comparison-policy | fixed (ADR 2996, amends ADR 1096; a field is in a Table 259 selection under its signed name or its current one; field_mdp_signed_name_spec_audit passes) | field rename after signing misclassified | 1 manifest failure
- 2038-T41 | pdf-signature,pdf-model | test-fixture-validity-gap | fixed (genuinely signed All, Exclude and `/P` witnesses from the audit's generator; pdf-model `signed_field_lock_shapes`, three tests) | signed-field lock fixtures invalid
- 2038-T67 | pdf-signature,viewer-core | test-coverage-gap | fixed (the audit's `signed_incremental_save_spec_audit` closes it and passes; nothing remains) | incremental signed writer test gap


## pdf-transform

### G080 — archive XFA packets non-stream silently dropped; test control: XFA attachment

- crates: pdf-transform
- source files: crates/pdf-transform/src/archive/attach.rs
- test files: crates/pdf-transform/tests/archive.rs?

- 2038-B58 | pdf-transform | malformed-input-integrity | fixed (ADR 2997; Annex K's packets read as string-and-stream pairs, else refused by name; the archive test passes) | archive XFA packets non-stream silently dropped | 1 manifest failure
- 2038-T112 | pdf-transform | test-assurance-gap | fixed (the audit's strengthened test passes; its malformed-array remainder is B58's test) | test control: XFA attachment

### G081 — PDF/A halftone transfer loss

- crates: pdf-transform
- source files: crates/pdf-transform/src/archive/halftones.rs, crates/pdf-transform/src/archive/rewrite/values.rs
- test files: crates/pdf-transform/tests/archive.rs

- 2038-B77 | pdf-transform | behavior-gap | fixed (ADR 2998, amends ADR 2034; a threshold halftone carrying a transfer function is refused unless the transfer row removes it; the part 4 test passes by refusal) | PDF/A halftone transfer loss | 1 manifest failure
- 2038-T168 | pdf-transform | assertion-quality | fixed (the part 2 control passes with ADR 2998's refusal in place) | PDF/A halftone transfer loss

### G082 — hard cap silently merges

- crates: pdf-transform
- source files: crates/pdf-transform/src/archive/external.rs
- test files: crates/pdf-transform/tests/archive_external_components_spec_audit.rs

- 2038-B46 | pdf-transform | file-specification-component-limit | open (the merge is `pdf_model::file_spec::MAX_COMPONENTS` in `crates/pdf-model/src/file_spec.rs`, G049's file, held by round 2077; the fix is to stop merging the tail past 256, after that round merges) | hard cap silently merges | 1 manifest failure

### G083 — split drops contentless structure elements

- crates: pdf-transform
- source files: crates/pdf-transform/src/structure/hierarchy.rs
- test files: crates/pdf-transform/tests/warichu_split_spec_audit.rs

- 2038-B56 | pdf-transform | behavior-gap | fixed (ADR 2999; a contentless element is kept in place where its parent is; warichu_split_spec_audit passes) | split drops contentless structure elements | 1 manifest failure

### G084 — apply_protected silently ignores protection under archive; test control: protection reaches outputs

- crates: pdf-transform
- source files: crates/pdf-transform/src/lib.rs
- test files: crates/pdf-transform/tests/protect_dispatch_spec_audit.rs

- 2038-T109 | pdf-transform | test-assurance-gap | fixed (the audit's `protect_dispatch_spec_audit` passes; D56's refusal has its own test) | test control: protection reaches outputs
- 2038-D56 | pdf-transform | api-contract-gap | fixed (ADR 2997; `apply_protected` refuses protection for any verb but the five that write a whole file; archive `an_archive_given_output_protection_is_refused_rather_than_written_plain`) | apply_protected silently ignores protection under archive

### G085 — web capture omission test doc

- crates: pdf-transform
- source files: crates/pdf-transform/src/archive/mod.rs?
- test files: crates/pdf-transform/tests/split.rs?

- 2038-D32 | pdf-transform | qualified-writer-rationale | fixed (`merge::is_web_capture`'s comment gives the half-structure reason for every version and §3.15 as PDF 2.0's added one; the audit corrected the two test comments) | web capture omission test doc

### G086 — date writer shares parser with test oracle

- crates: pdf-transform
- source files: crates/pdf-transform/src/update.rs
- test files: crates/pdf-transform/tests/date_spec_audit.rs

- 2038-T01 | pdf-transform | test-oracle-gap | fixed (pdf-transform `date_grammar`: §7.9.4 read without `Date::parse`, applied to every date the writer emits) | date writer shares parser with test oracle

### G087 — archive embedding filter parameters

- crates: pdf-transform
- source files: crates/pdf-transform/src/attachments.rs?
- test files: crates/pdf-transform/tests/archive.rs?

- 2038-T154 | pdf-transform | assertion-quality | fixed (the audit's archive test closes it and passes) | archive embedding filter parameters


## pdf-archive

### G088 — incomplete survey yields Met; transparency survey over-counts; transparency survey test gap

- crates: pdf-archive
- source files: crates/pdf-archive/src/survey.rs
- test files: crates/pdf-archive/tests/incomplete_transparency_spec_audit.rs, crates/pdf-archive/tests/transparency_detection_spec_audit.rs, crates/pdf-archive/tests/transparency_resources_spec_audit.rs

- 2038-B35 | pdf-archive | implementation-detection-gap | fixed (ADR 2994; Table 166's `/CA` counts only without an appearance stream; transparency_detection_spec_audit passes) | transparency survey over-counts | 1 manifest failure
- 2038-B37 | pdf-archive | false-validation-result | fixed (ADR 2994; a stopped walk reports the pages it did not see as transparent; incomplete_transparency_spec_audit's four pass) | incomplete survey yields Met | 4 manifest failures
- 2038-T50 | pdf-archive | assertion-coverage-gap | fixed (the audit's `transparency_resources_spec_audit` closes it and passes) | transparency survey test gap
- 2038-D24 | pdf-archive | provisional-source-interpretation | fixed (ADR 2994; Annex Q.2 counts elements rendered on the page, so a soft mask's group walked at `gs` no longer marks the page; an element painted under the mask still does; pdf-archive `soft_mask_population`, two tests) | transparency survey over-counts

### G089 — archive witness gate skips missing named files

- crates: pdf-archive
- source files: none (test or document only)
- test files: crates/pdf-archive/tests/witnesses.rs

- 2038-D52 | pdf-archive | fixture-policy-hardening | fixed (the audit's gate change; `witnesses.rs`'s module comment states the present policy) | archive witness gate skips missing named files

### G090 — test claims too strong

- crates: pdf-archive
- source files: crates/pdf-archive/src/lib.rs?
- test files: crates/pdf-archive/tests/verdict.rs?

- 2038-T110 | pdf-archive | test-assurance-gap | fixed (the audit's restricted-source test closes it and passes) | test claims too strong


## pdf-ccitt

### G091 — CCITT EndOfBlock rows

- crates: pdf-ccitt, pdf-model
- source files: crates/pdf-ccitt/src/lib.rs?
- test files: crates/pdf-model/tests/ccitt_eob_rows_spec_audit.rs

- 2038-T152 | pdf-ccitt,pdf-model | assertion-quality | fixed (the audit's `ccitt_eob_rows_spec_audit` closes it and passes) | CCITT EndOfBlock rows


## render-cpu

### G092 — degenerate stroke caps composited separately; stroke geometry test gap; stroke width under unequal scale

- crates: pdf-model, render-cpu, render-gpu, render-raster
- source files: crates/render-cpu/src/stroke.rs, crates/render-raster/src/stroke.rs
- test files: crates/pdf-model/tests/stroke_composition_pdf_spec_audit.rs, crates/render-cpu/tests/stroke_fill_geometry_spec_audit.rs, crates/render-cpu/tests/transparent_degenerate_spec_audit.rs, crates/render-gpu/tests/stroke_composition_spec_audit.rs, crates/render-raster/tests/anisotropic_stroke_width_spec_audit.rs, crates/render-raster/tests/near_similarity_stroke_spec_audit.rs, crates/render-raster/tests/stroke_composition_spec_audit.rs

- 2038-B39 | render-cpu,render-raster,render-gpu,pdf-model | same-object-double-compositing | open | degenerate stroke caps composited separately | 5 manifest failures
- 2038-B59 | render-raster | rendering-behavior-gap | open | stroke width under unequal scale | 2 manifest failures
- 2038-T53 | render-cpu | assertion-coverage-gap | open | stroke geometry test gap
- 2038-T111 | render-raster | independent-oracle-control | open | stroke width under unequal scale

### G093 — degenerate shading geometry painting

- crates: pdf-render, render-cpu
- source files: crates/pdf-render/src/shading.rs, crates/render-cpu/src/shading.rs
- test files: crates/pdf-model/tests/degenerate_shading_bbox_spec_audit.rs, crates/pdf-model/tests/degenerate_shading_geometry_spec_audit.rs, crates/render-raster/tests/degenerate_shading_bbox_spec_audit.rs

- 2038-B70 | render-cpu,pdf-render | behavior-gap | open | degenerate shading geometry painting | 1 manifest failure
- 2038-B71 | render-cpu,pdf-render | behavior-gap | open | degenerate shading geometry painting | 1 manifest failure
- 2038-T126 | pdf-render,render-cpu | assertion-strengthening | open | degenerate shading geometry painting


## render-raster

### G094 — degenerate shading geometry painting

- crates: render-raster
- source files: crates/render-raster/src/scene/shadings.rs
- test files: crates/render-raster/tests/degenerate_shading_geometry_spec_audit.rs

- 2038-T128 | render-raster | assertion-strengthening | open | degenerate shading geometry painting

### G095 — test assertion: stroke-fit coverage

- crates: render-raster
- source files: crates/render-raster/src/lib.rs?
- test files: crates/render-raster/tests/stroke_adjustment.rs

- 2038-T37 | render-raster | test-assertion-gap | open | test assertion: stroke-fit coverage


## render-gpu

### G096 — GPU shading tests overstate pixel-centre rule

- crates: render-gpu
- source files: crates/render-gpu/tests/headless_gpu.rs?
- test files: crates/render-gpu/tests/headless_gpu.rs?

- 2038-D54 | render-gpu | source-rationale-overreach | open | GPU shading tests overstate pixel-centre rule

### G097 — test oracle too weak or vacuous

- crates: render-gpu, render-raster
- source files: none (test or document only)
- test files: crates/render-gpu/tests/headless_gpu.rs, crates/render-raster/tests/headless_quorra.rs

- 2038-T104 | render-gpu,render-raster | test-execution-gap | open | test oracle too weak or vacuous


## pdf-script

### G098 — engine realm lacks Matrix2D constructor

- crates: pdf-script
- source files: crates/pdf-script/src/engine/mod.rs
- test files: crates/pdf-script/tests/matrix2d_reference_spec_audit.rs

- 2038-B52 | pdf-script | adopted-script-reference-surface-gap | open | engine realm lacks Matrix2D constructor | 1 manifest failure

### G099 — indirect JS stream test gap

- crates: pdf-script
- source files: crates/pdf-script/src/lib.rs?
- test files: crates/pdf-script/tests/hook.rs

- 2038-T65 | pdf-script | test-coverage-gap | open | indirect JS stream test gap


## viewer-core

### G100 — GoToE parent target walk; URI action IsMap; empty Fields array presence lost; initial open actions run in wrong order; non-inheritable field keys read via parents; page open action trigger URI resolution; remote destination resolution; remote destination view not applied; rendition script ignored; test control: URI IsMap; test control: action Next cycle; test control: text-markup popup

- crates: pdf-model, viewer-core
- source files: crates/pdf-model/src/action.rs, crates/pdf-model/src/submission.rs, crates/viewer-core/src/interact.rs, crates/viewer-core/src/open.rs
- test files: crates/pdf-model/tests/empty_fields_array_presence_spec_audit.rs, crates/pdf-model/tests/tm_coordinate_name_spec_audit.rs, crates/pdf-model/tests/tm_html_export_spec_audit.rs, crates/viewer-core/tests/action_triggers_spec_audit.rs?, crates/viewer-core/tests/cross_document_destination_view_spec_audit.rs, crates/viewer-core/tests/embedded_parent_relative_spec_audit.rs, crates/viewer-core/tests/initial_open_order_spec_audit.rs, crates/viewer-core/tests/next_repeated_and_cycle_spec_audit.rs, crates/viewer-core/tests/remote_named_destinations_spec_audit.rs, crates/viewer-core/tests/rendition_preconditions_spec_audit.rs, crates/viewer-core/tests/text_markup_popup_spec_audit.rs, crates/viewer-core/tests/uri_ismap_click_spec_audit.rs, crates/viewer-core/tests/uri_ismap_outline_spec_audit.rs

- 2038-B22 | viewer-core | behavior-mismatch | fixed (ADR 2964; initial_open_order_spec_audit: the open action is performed in `Viewer::process` before page one's /O) | initial open actions run in wrong order | 1 manifest failure
- 2038-B23 | viewer-core | behavior-mismatch | fixed (ADR 2964; cross_document_destination_view_spec_audit ×2: `place_remote` and `jump_into` keep /D's Table 149 view) | remote destination view not applied | 2 manifest failures
- 2038-B49 | viewer-core,pdf-model | rendition-script-priority | fixed (ADR 2988; rendition_preconditions_spec_audit, rendition_scripts; round 2077) | rendition script ignored | 1 manifest failure
- 2038-B60 | pdf-model | behavior-gap | fixed (ADR 2965; tm_coordinate_name_spec_audit: `submission::mapping_name` reads the terminal field only) | non-inheritable field keys read via parents | 1 manifest failure
- 2038-B61 | pdf-model | behavior-gap | fixed (ADR 2965; tm_html_export_spec_audit: the HTML Form body writes /TM) | non-inheritable field keys read via parents | 1 manifest failure
- 2038-B66 | pdf-model | behavior-gap | fixed (ADR 2965; empty_fields_array_presence_spec_audit ×2: `fields: Option<Vec<ResetTarget>>`; the type change edits the literal in inherited_reset_defaults_audit.rs to `fields: None`) | empty Fields array presence lost | 2 manifest failures
- 2038-B74 | pdf-model,viewer-core | behavior-gap | fixed (ADR 2964; embedded_parent_relative_spec_audit ×2: `EmbeddedGoTo::target_in` walks a lineage, `Open::parents`) | GoToE parent target walk | 2 manifest failures
- 2038-T13 | viewer-core | test-coverage-gap | fixed (text_markup_popup_spec_audit, plant-checked: dropping `interact::activate_object`'s popup route fails it; it does not pin `popup::opens_a_window`, whose explicit-/Popup case it bypasses) | test control: text-markup popup
- 2038-T28 | viewer-core | test-coverage-gap | fixed (next_repeated_and_cycle_spec_audit, plant-checked: keeping every seen object in `script_sites::walk` fails it) | test control: action Next cycle
- 2038-T29 | viewer-core | test-coverage-gap | fixed (uri_ismap_click_spec_audit ×2, plant-checked: `Uri::at_position` ignoring /IsMap fails both) | test control: URI IsMap
- 2038-T118 | pdf-model | assertion-strengthening | fixed (page_aa_indirect_reference_spec_audit, plant-checked: an unresolved indirect page /AA fails it) | page open action trigger URI resolution
- 2038-T158 | viewer-core | assertion-quality | fixed (remote_named_destinations_spec_audit, plant-checked: `RemoteGoTo::page_in` declining a Name or string /D fails it) | remote destination resolution
- 2038-T159 | pdf-model,viewer-core | assertion-quality | fixed (attachment_child_route_spec_audit, plant-checked: an off-by-one /A in `attached_file` fails it) | GoToE parent target walk
- 2038-T162 | viewer-core | assertion-quality | fixed (uri_ismap_outline_spec_audit, plant-checked: coordinates appended to the outline's URI fail it) | URI action IsMap

### G101 — revocation time basis misapplied

- crates: pdf-signature, viewer-core
- source files: crates/pdf-signature/src/revocation.rs, crates/pdf-signature/src/timestamp.rs
- test files: crates/viewer-core/tests/current_crl_report_spec_audit.rs, crates/viewer-core/tests/historical_ocsp_report_spec_audit.rs

- 2038-B41 | viewer-core,pdf-signature | timestamp-time-selection | fixed (ADR 2968; notes::timestamp_revocation_spec_audit: `signature_timestamps_instant` declines a compromised or earlier-revoked TSA path) | revocation time basis misapplied | 1 manifest failure
- 2038-B44 | viewer-core,pdf-signature | current-crl-applicability | fixed (ADR 2968; current_crl_report_spec_audit: `Basis::CurrentTime` adds thisUpdate's end) | revocation time basis misapplied | 1 manifest failure
- 2038-B47 | viewer-core,pdf-signature | historical-ocsp-applicability | fixed (ADR 2968; historical_ocsp_report_spec_audit: `Basis::PastTime` adds the producedAt condition) | revocation time basis misapplied | 1 manifest failure
- 2038-D36 | viewer-core,pdf-signature | qualified-static-timing-gap | fixed (ADR 2968: `revocation::Basis` and `notes::trust_as_of` state the two clauses' windows) | revocation time basis misapplied

### G102 — image XObject measure ignored by query; measurement number formatting; measurement numeric test gap

- crates: pdf-model, viewer-core
- source files: crates/pdf-model/src/measurement.rs, crates/viewer-core/src/viewer/geometry.rs
- test files: crates/viewer-core/tests/image_xobject_measurement_spec_audit.rs, crates/viewer-core/tests/measurement_format_spec_audit.rs, crates/viewer-core/tests/measurement_numeric_spec_audit.rs

- 2038-B73 | viewer-core,pdf-model | behavior | open (not reached in rounds 2073, 2076 and 2077. What it needs: Table 269's /LPTS unit square "is mapped to the rectangular bounds of the ... image XObject", which is the image's own unit square, so the CTM at its `Do` is the mapping; the interpreter reports each image XObject's CTM and /Measure (crates/pdf-model/src/content/xobject.rs), the viewer keeps them with the interpreted page, and viewer-core `geometry::measured` and `located::locate` read them beside the page's /VP, with /PCSM's "XObject position coordinates" taken in image space) | image XObject measure ignored by query | 1 manifest failure
- 2038-T52 | viewer-core,pdf-model | assertion-coverage-gap | fixed (measurement_numeric_spec_audit, plant-checked: `traced` taking the last point's viewport fails it; its viewports do not overlap, so /VP's last-containing order is not pinned) | measurement numeric test gap
- 2038-T153 | viewer-core,pdf-model | assertion-quality | fixed (measurement_format_spec_audit, plant-checked: a literal '.' for /RD fails it) | measurement number formatting

### G103 — requirements minimum version ignored; requirements processing user-override doc

- crates: pdf-model, viewer-core
- source files: crates/pdf-model/src/requirements.rs, crates/viewer-core/src/notes.rs
- test files: crates/viewer-core/tests/requirements_version_spec_audit.rs

- 2038-B38 | viewer-core,pdf-model | ignored-minimum-requirement-version | fixed (ADR 2966; requirements_version_spec_audit ×2: `Requirement::unmet` compares a PDF /V with `Version::SUPPORTED`) | requirements minimum version ignored | 2 manifest failures
- 2038-D27 | viewer-core | policy-source-distinction | fixed (ADR 2966: notes.rs says the processing level decides the total's consequence, never which requirements count) | requirements processing user-override doc

### G104 — signature verification deferred on open

- crates: viewer-core
- source files: crates/viewer-core/src/open/deferred.rs, crates/viewer-core/src/viewer/opening.rs
- test files: none located

- 2038-B17 | viewer-core | static-behavior-gap | fixed (ADR 2969; pades_signature_verified_at_open: the first `Command::Presented` of a PAdES-signed document reports its verification) | signature verification deferred on open

### G105 — remote destination view not applied

- crates: viewer-core
- source files: crates/viewer-core/src/open/destinations.rs
- test files: crates/viewer-core/tests/headless.rs

- 2038-B24 | viewer-core | behavior-mismatch-and-wrong-existing-oracle | fixed (ADR 2964; headless ×3: `centred_corner` for /FitR and /FitB) | remote destination view not applied | 3 manifest failures

### G106 — VisiblePages considers one page

- crates: pdf-model, viewer-core
- source files: crates/viewer-core/src/viewer/answers.rs
- test files: crates/viewer-core/tests/optional_content_visible_pages_spec_audit.rs

- 2038-B51 | viewer-core,pdf-model | visible-pages-layer-filter | fixed (ADR 2967; optional_content_visible_pages_spec_audit: `layers` unions every placed page) | VisiblePages considers one page | 1 manifest failure

### G107 — rendition preconditions doc stale

- crates: pdf-model, viewer-core
- source files: crates/pdf-model/src/view/actions.rs?
- test files: none located

- 2038-D40 | viewer-core,pdf-model | stale-screen-rendition-scope | fixed (ADR 2964: §12.5.6.18's ledger note says the /P bullet is executed by `Rendition::unplaced`) | rendition preconditions doc stale

### G108 — test control: omitted Dur

- crates: viewer-core
- source files: crates/viewer-core/src/presentation.rs?
- test files: crates/viewer-core/tests/omitted_page_duration_spec_audit.rs

- 2038-T21 | viewer-core | test-coverage-gap | fixed (omitted_page_duration_spec_audit, plant-checked: an absent /Dur read as 0 fails it) | test control: omitted Dur

### G109 — test control: Annex O URI and highlight geometry

- crates: viewer-core
- source files: crates/viewer-core/src/select.rs?
- test files: crates/viewer-core/tests/annex_o_coordinate_policy_spec_audit.rs, crates/viewer-core/tests/annex_o_structure_uri_spec_audit.rs

- 2038-T38 | viewer-core | public-integration-test-gap | fixed (annex_o_structure_uri_spec_audit and annex_o_coordinate_policy_spec_audit, plant-checked: no percent-decoding fails the first, a highlight skipping the page transform the second) | test control: Annex O URI and highlight geometry

### G110 — test control: open-event missing-object notes

- crates: viewer-core
- source files: crates/viewer-core/src/lib.rs?
- test files: crates/viewer-core/tests/initial_open_order_spec_audit.rs

- 2038-T84 | viewer-core | test-coverage-gap | fixed (objects_not_understood::opening_an_unhandled_object_reports_it_to_the_host_before_render_ready, plant-checked at `install`'s page report; the open's own Reported blocks in `process` are not pinned by it) | test control: open-event missing-object notes


## viewer-host

### G111 — collection details view cells incomplete; test control: Info vs XMP properties

- crates: viewer-host
- source files: crates/viewer-host/src/panel.rs
- test files: crates/viewer-host/tests/collection_schema_cells_audit.rs, crates/viewer-host/tests/metadata_sources_spec_audit.rs

- 2038-B05 | viewer-host | behavior-gap | open | collection details view cells incomplete | 3 manifest failures
- 2038-B06 | viewer-host | behavior-gap | open | collection details view cells incomplete | 1 manifest failure
- 2038-T16 | viewer-host | test-coverage-gap | open | test control: Info vs XMP properties

### G112 — required-empty field submit diagnostics

- crates: viewer-host
- source files: crates/viewer-host/src/submit.rs?
- test files: crates/viewer-host/tests/required_empty_field_report_and_policy.rs

- 2038-T121 | viewer-host | assertion-strengthening | open | required-empty field submit diagnostics


## viewer-accessibility

### G113 — mixed K logical order lost

- crates: viewer-accessibility
- source files: crates/viewer-accessibility/src/tree.rs
- test files: crates/viewer-accessibility/tests/interleaved_struct_content_spec_audit.rs

- 2038-B43 | viewer-accessibility | accessibility-logical-order | open | mixed K logical order lost | 1 manifest failure

### G114 — Ruby role mapping ADR stale

- crates: viewer-accessibility
- source files: doc/adr/2384-table-380s-decoration-and-position-reach-the-accessibility-tree-on-the-elements-own-text.md
- test files: none located

- 2038-D23 | viewer-accessibility | adr-wording-gap | open | Ruby role mapping ADR stale

### G115 — artifact classification test reach doc

- crates: viewer-accessibility
- source files: crates/viewer-accessibility/src/bridge.rs?
- test files: crates/viewer-accessibility/tests/tree.rs

- 2038-D26 | viewer-accessibility | consumer-evidence-boundary | open | artifact classification test reach doc

### G116 — Ruby role mapping test gap

- crates: viewer-accessibility
- source files: crates/viewer-accessibility/src/role.rs
- test files: crates/viewer-accessibility/tests/tree.rs?

- 2038-T49 | viewer-accessibility | assertion-coverage-gap | open | Ruby role mapping test gap


## viewer-ui

### G117 — Windows HideWindowUI tests lack control observation

- crates: viewer-ui
- source files: none (test or document only)
- test files: none located

- 2038-D53 | viewer-ui | test-integration-residual | open | Windows HideWindowUI tests lack control observation

### G118 — outline item style rendering

- crates: pdf-model, viewer-ui
- source files: crates/pdf-model/src/outline.rs?
- test files: crates/viewer-ui/tests/outline_style_spec_audit.rs

- 2038-T134 | viewer-ui,pdf-model | assertion-strengthening | open | outline item style rendering

### G119 — test control: outline colour pixels

- crates: viewer-ui
- source files: crates/viewer-ui/src/outline.rs?
- test files: crates/viewer-ui/tests/outline_colour_spec_audit.rs

- 2038-T71 | viewer-ui | test-coverage-gap | open | test control: outline colour pixels


## viewer-confined

### G120 — test control: confined saved-event

- crates: viewer-confined
- source files: crates/viewer-confined/src/protocol/?
- test files: crates/viewer-confined/tests/confined.rs?

- 2038-T114 | viewer-confined | assertion-strengthening | open | test control: confined saved-event


## viewer-gtk

### G121 — test control: print preference settings

- crates: viewer-gtk, viewer-qt
- source files: crates/viewer-gtk/src/host/printing.rs
- test files: crates/viewer-gtk/src/host/printing/print_defaults_settings_spec_audit.rs, crates/viewer-qt/src/host/tests/print_defaults_spec_audit.rs

- 2038-T89 | viewer-gtk,viewer-qt | test-coverage-gap | open | test control: print preference settings


## confined-transport

### G122 — test claims too strong

- crates: confined-transport, viewer-confined
- source files: crates/confined-transport/src/?
- test files: crates/viewer-confined/tests/confined.rs?

- 2038-T106 | confined-transport,viewer-confined | test-assurance-gap | open | test claims too strong


## doc

### G123 — audit note citation error; doc: EC3 renumbering errata; doc: errata-read #309 description

- crates: doc
- source files: doc/errata-read.md
- test files: none located

- 2038-D01 | doc | citation-drift | open | doc: EC3 renumbering errata
- 2038-D11 | doc | source-provenance-gap | open | doc: errata-read #309 description
- 2038-D65 | doc | source-crosswalk-citation | open | audit note citation error
- 2038-D66 | doc | audit-source-correction | open | audit note citation error
- 2038-D67 | doc | documentation-gap | open | audit note citation error

### G124 — doc: MissingWidth ledger explanation; doc: inline image dimension rounding; doc: ledger test reference missing; doc: ledger text-state rows stale; doc: stroke adjustment ledger prose

- crates: doc, viewer-core
- source files: doc/conformance/ledger.toml
- test files: crates/viewer-core/tests/annex_o_open_parameters_audit.rs

- 2038-D06 | doc | stale-ledger-policy-description | open | doc: inline image dimension rounding
- 2038-D09 | doc | stale-ledger-description-and-evidence-links | open | doc: ledger text-state rows stale
- 2038-D10 | doc | ledger-erratum-qualification-gap | open | doc: MissingWidth ledger explanation
- 2038-D13 | doc | ledger-description-drift | open | doc: stroke adjustment ledger prose
- 2038-D14 | doc,viewer-core | ledger-test-reference-gap | open | doc: ledger test reference missing

### G125 — errata inventory mislocates entries; errata per-issue indexes omit grouped markers

- crates: doc
- source files: doc/reviews/2038-specification-audit/?
- test files: none located

- 2038-D46 | doc | errata-inventory-gap | open | errata per-issue indexes omit grouped markers
- 2038-D50 | doc | stale-source-and-consumer-rationale | open | errata inventory mislocates entries

### G126 — doc: collection initial-document ADR wording

- crates: doc
- source files: doc/adr/0202-a-collection-is-how-a-document-arranges-its-files.md, doc/adr/0231-a-collection-opens-on-one-document-and-nobody-could-say-which.md
- test files: none located

- 2038-D02 | doc | interpretation-follow-up | open | doc: collection initial-document ADR wording

### G127 — doc: ADR0601 premise outdated

- crates: doc
- source files: doc/adr/0601-two-accepted-errata-that-cannot-both-be-applied.md
- test files: none located

- 2038-D03 | doc | decision-premise-correction | open | doc: ADR0601 premise outdated

### G128 — doc: ADR2811 PLRM bitshift overstated

- crates: doc
- source files: doc/adr/2811-bitshift-shifts-zeros-in-from-either-end.md
- test files: none located

- 2038-D04 | doc | decision-premise-qualification | open | doc: ADR2811 PLRM bitshift overstated

### G129 — doc: Ink appearance EC3#444

- crates: doc, pdf-model
- source files: crates/pdf-model/src/content/annotations.rs?, doc/conformance/ledger.toml
- test files: crates/pdf-model/tests/ink_annotation_spec_audit.rs

- 2038-D12 | doc,pdf-model | source-ledger-rationale-gap | open | doc: Ink appearance EC3#444

### G130 — Table 285 BU/MH EC3 redirect conflict

- crates: doc
- source files: doc/todo/ (media data BU ADR)?
- test files: none located

- 2038-D44 | doc | source-inconsistency | open | Table 285 BU/MH EC3 redirect conflict

### G131 — historical errata synopsis mislocates correction

- crates: doc
- source files: doc/errata ADR for #193?
- test files: none located

- 2038-D47 | doc | source-description-error | open | historical errata synopsis mislocates correction

### G132 — ADR cites deleted object-zero prohibition

- crates: doc
- source files: doc/adr (incremental free-entry rationale)?
- test files: none located

- 2038-D48 | doc | superseded-design-rationale | open | ADR cites deleted object-zero prohibition

### G133 — ledger keeps deleted EC3 prohibition

- crates: doc
- source files: doc/conformance ledger rows 12.5.1/12.5.6.18?
- test files: none located

- 2038-D49 | doc | stale-ledger-source | open | ledger keeps deleted EC3 prohibition

### G134 — ADR FileSelect timing rationale stale

- crates: doc
- source files: crates/1240 (FileSelect)?, doc/adr/1216
- test files: none located

- 2038-D55 | doc | decision-revisit | open | ADR FileSelect timing rationale stale

### G135 — archive dynamic-XFA rationale stale

- crates: doc
- source files: doc/adr (archive dynamic XFA)?
- test files: none located

- 2038-D57 | doc | decision-revisit | open | archive dynamic-XFA rationale stale

### G136 — ADR interval argument unproven

- crates: doc
- source files: doc/adr/0030 (raster)?
- test files: none located

- 2038-D58 | doc | decision-rationale-qualification | open | ADR interval argument unproven

### G137 — ADR premise expired by confined editing

- crates: doc
- source files: doc/adr/1190?
- test files: none located

- 2038-D59 | doc | decision-premise-qualification | open | ADR premise expired by confined editing

### G138 — ledger row stale

- crates: doc
- source files: doc/ledger row 7.7.2/12.6.3?
- test files: none located

- 2038-D60 | doc | ledger-narrative-drift | open | ledger row stale

### G139 — degenerate shading geometry painting

- crates: doc
- source files: doc/adr/2356-the-oracle-draws-its-own-ramps-at-each-pixels-t-as-the-level-nearest-the-ramps-colour.md
- test files: none located

- 2038-D61 | doc | decision-qualification | open | degenerate shading geometry painting

### G140 — Type3 font glyph handling

- crates: doc
- source files: doc/adr/0914-a-type-3-font-matrix-written-to-two-decimal-places.md
- test files: none located

- 2038-D62 | doc | decision-qualification | open | Type3 font glyph handling

### G141 — ADR table citation error

- crates: doc
- source files: doc/adr/0473-the-clock-two-windows-did-not-have.md
- test files: none located

- 2038-D63 | doc | documentation-gap | open | ADR table citation error

### G142 — tiling pattern lattice spacing

- crates: doc
- source files: doc/adr/1080-a-tiling-lattice-is-snapped-where-a-device-pixel-is-stated.md
- test files: none located

- 2038-D64 | doc | decision-qualification | open | tiling pattern lattice spacing

### G143 — audit review artifact integrity

- crates: doc
- source files: doc/reviews/2038-specification-audit.md?
- test files: none located

- 2038-D68 | doc | documentation-gap | open | audit review artifact integrity

## Failures with no finding

- pdf-model::named_page_geometry_spec_audit apref_page_rotation_crop_and_inherited_resources_reach_the_widget_pixels
  (an audit test, beside G061's T113) fails on the merged tree. Main's c77df9e2, after the audit's base, reports a
  simple font with no /Widths as laid out by this viewer's choice (ISO 32000-2 Table 109 requires /Widths outside
  the standard 14), and the fixture's InheritedOnly font has none, so `is_complete()` is false. The fixture is
  invalid for the point it tests; the round that takes G061 gives the font /Widths (A868), not the code a change.
  Status: open.

## Conformance gates the merged audit files fail

`cargo test -p conformance` on the merged tree fails five tests, every one on a file the audit added or changed;
round 2070 edited none of them. Each goes to the group that owns the file.

- `every_citation_names_a_clause_that_exists`: `crates/pdf-syntax/src/crypt.rs:1940` writes a `§` after RFC 4013
  (write "RFC 4013 section N"). G001.
- `every_table_designation_names_a_table_the_standard_captions`:
  `crates/pdf-colour/tests/calculator_functions_spec_audit.rs:161` names "Table 3.1", which ISO 32000-2 does not
  caption (name the other standard before the word). G001.
- `no_comment_names_a_round_by_number`: `crates/pdf-syntax/tests/public_key_witnesses.rs:47` names "round 2038"
  (ADR 1023). G001.
- `the_ledger_agrees_with_the_standard_and_with_the_tree`: §14.9.2.2's row names
  `crates/pdf-model/tests/accessibility.rs::an_empty_language_identifier_states_nothing`, which the audit's change to
  that file removed. G017.
- `every_record_since_the_rule_states_its_gates`: `doc/history/2038-specification-evidence-audit.md` has a Gates
  paragraph with no exit status (ADR 1499). That record is the owner's and is not rewritten; the gate's held list or
  the owner decides.
