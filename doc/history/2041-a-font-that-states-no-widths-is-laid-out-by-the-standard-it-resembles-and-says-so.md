# 2041 — A font that states no widths is laid out by the standard it resembles, and says so

Session 2041, a single ledger round in the audit worktree. Rows 9.6.2.1 and 9.8.1; A859; ADRs 2898, 2899; Q868.

**The slice.** A859's order for a simple font outside the fourteen stating no `/Widths` (Table 109 requires it):
a program's own advances, the resembling standard metrics, then `/MissingWidth`; reported, and archived under
`layout-from-this-viewer`.

**The premise.** Unbuilt, and only in part effort-only. A substitute (no program, ADR 0007; an unreadable one,
ADR 2870; no descriptor, A687) took the substitute face's own advance for a code the resembling metrics did not
name, which is round 2027's finding. The order for an embedded program and for a metrics-only font was built and
failed three owner audit tests that pin `/MissingWidth` (`the_program_states_an_advance_of_its_own`, two in
`refused_font_metrics.rs`); it was withdrawn and asked as Q868, recommending (a).

**Built.** `metrics::simple_widths` gives a substitute outside the fourteen the published metrics and then
`/MissingWidth`, never the face's advance, and returns `SuppliedWidths`; `LoadedFont::supplied_widths()` carries
it. The interpreter reports it once per font per page as `Unsupported::Font`, except for this reader's own
stand-in in a stream nothing names (ADR 0112). The archive: the embedding route writes a font with a descriptor
and no `/Widths` under `layout-from-this-viewer` at the window's advances, restated before the proofs;
`fonts/simple-font-states-its-widths` is a per-document condition the three profiles grant or stop at; R09 says so.

**Tests.** `unstated_widths.rs` (three), `substituted_width_tests` (one rewritten, one control),
`layout_from_this_viewer.rs` (two: window and archive agree on every advance), `type1_widths.rs` (one asks for
the grant); three rich-text fixtures state the `/Widths` Table 109 requires. **Plant**: the face's advances
restored and the report dropped fail `a_substitute_with_no_widths_keeps_the_resembling_standard_metrics` and
`a_font_with_no_widths_is_reported`.

**Gates.** Over `pdf-font`, `pdf-model`, `pdf-transform`: `cargo fmt --check` exit 0; clippy `-D warnings
--all-targets` exit 0; nextest 4264 of 4264 passed; rustdoc `-D warnings --document-private-items` exit 0. `cd fuzz
&& cargo check --bins` exit 0. Conformance after `git add -N`: exit 101 on this record's gates paragraph, then the
whole crate exit 0. Tier 2: `corpus` 101 as HEAD's (the same pair joined) once six documents were admitted
beside §9.6.2.1 — `issue5470`, `issue6069`, `issue6106`, `issue6108`, `issue2391-2`, `issue18099_reduced`, each
now reporting its font's missing `/Widths`; `raster_golden` 101 as HEAD's, its 23 first pages, once those six
pages' reports were re-held (their rasters did not move); `pdf-archive` `corpus` 0; `pdf-transform`
`archive_corpus` 0, `issue5470` and safedocs `0000490` newly naming `fonts/simple-font-states-its-widths`;
`state.sh docs` exit 0.

**Left of the rows.** 9.6.2.1 and 9.8.1 stay `implemented`: an embedded program and a metrics-only font that
state no `/Widths` advance by `/MissingWidth` for every code, unreported, until Q868. R09's re-census is the walks'
populations, named above.
