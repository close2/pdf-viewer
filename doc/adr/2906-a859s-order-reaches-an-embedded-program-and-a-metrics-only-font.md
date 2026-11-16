# 2906 — A859's order reaches an embedded program and a metrics-only font

Session 2045, a font and PDF/A round in the audit worktree, on the owner's answer `doc/questions/A868`.
Status: accepted and built. In `pdf-font`: `metrics.rs`'s `simple_widths`, `unprogrammed_widths`,
`program_widths` (a Type 1 program's `hsbw`) and `SuppliedWidths::from_program`;
`loading/opening.rs`'s `published_metrics`; `loading.rs`'s `StatedMetrics::read`. In `pdf-model`:
`content/font.rs`, the metrics-only route's `note_supplied_widths`.
Tests: `crates/pdf-font/src/loading.rs`'s `the_program_states_an_advance_of_its_own`,
`crates/pdf-model/tests/refused_font_metrics.rs`'s `a_refused_program_falls_back_to_the_stated_missing_width`
and `a_refused_program_stating_no_width_advances_by_nothing` (the three the owner sanctioned),
`crates/pdf-font/src/metrics.rs`'s `substituted_width_tests` (a Type 1 program's own advances),
`crates/pdf-transform/tests/archive/layout_from_this_viewer.rs` (an embedded program, window and archive).
Context: ADR 2898 (the order, built for the substitute routes and held here), ADR 2899 (the archive),
ADR 1094 (the metrics-only font), ADR 0039 (Table 120's default against an invented half-em).
Clauses: ISO 32000-2 §9.6.2.1 (Table 109), §9.6.2.2, §9.8.1 (Table 120), §9.2.4.

## Context

ADR 2898 ordered the advances of a simple font outside §9.6.2.2's fourteen that states no `/Widths`
— the program's own, the resembling standard font's published metrics, then `/MissingWidth` — and
built it only where a substitute is drawn, because three tests of the owner's audit pinned Table
120 for every code on the other two routes. Q868 asked whether those three may change; A868 answers
yes, option (a).

## Decision

1. **One order on every route.** `simple_widths` takes, for a font outside the fourteen whose
   `/Widths` states nothing: the embedded program's advance for each code that reaches a glyph
   other than `.notdef` (Table 109: "These widths shall be consistent with the actual widths given
   in the font program", so the program's number is the file's own second statement of the width);
   then the published metric of the standard font the name and descriptor resemble
   (`substitute::Request::derive`, now asked of every font outside the fourteen); then
   `/MissingWidth`, stated or 0. A substitute's face is no program of the file's and is no step
   (ADR 2898). A Type 1 program's advances are its `hsbw` widths, read from the parse the font keeps.
2. **A metrics-only font** (ADR 1094) has no program to ask: the published metrics answer the codes
   its `/Encoding` names, then `/MissingWidth` (`metrics::unprogrammed_widths`).
3. **It is said on every route.** `SuppliedWidths` counts the codes the program answered beside
   those the metrics answered; the interpreter notes `Unsupported::Font` for the metrics-only route
   as it does for the others, so the page is incomplete (trap 5).
4. **The archive follows by construction**: `font_widths.rs` writes a descriptor-bearing font's
   missing array at `LoadedFont::advance` and marks it `layout-from-this-viewer` wherever
   `supplied_widths` is present, so an embedded program now takes the loss, and the array written
   is the program's own advances, which section 6.2.11.5 of ISO 19005-2 asks the dictionary to
   agree with. A metrics-only font is refused by the archive as before: its program is unreadable,
   which no width answers.
5. **The three audit tests** now assert A859's numbers (the program's 0.5 em; Helvetica's 667 for
   `A` and `/MissingWidth` for a code no metric names) and the report beside them, each citing ADR
   2898 and A859. No other audit test changed.

## Consequences

An embedded program that states no `/Widths` is laid out by its own advances, where it drew every
glyph on one point before; such pages are incomplete and say why, and the round's record names the
corpus pages that moved.
