# 2899 — The archive writes a font's missing widths as this viewer's layout, under the loss

Session 2041, the archive half of ADR 2898, on the owner's answers `doc/questions/A859` and
`doc/questions/A687`.
Status: accepted and built. In `pdf-transform`: `archive/font_widths.rs` (`WidthArrays::this_viewers`,
`lays_out_by_this_viewer`), `archive/prepare.rs`'s `Prepared::widths_from_this_viewer`,
`archive/decision.rs`'s `answer_of` arm and the loss's sentence, `archive/decision/fonts.rs`'s
`CONDITIONAL` row for `fonts/simple-font-states-its-widths` and its remedy sentence,
`archive/fonts.rs`'s embedding route; `doc/profiles/as-if-printed.toml`,
`state-every-loss.toml` and `as-shown.toml`.
Tests: `crates/pdf-transform/tests/archive/type1_widths.rs`
(`a_truetype_font_drawn_only_in_mode_three_with_no_entries_is_given_a_descriptor_at_both_parts`),
`crates/pdf-transform/tests/archive/layout_from_this_viewer.rs` (the last two).
Context: ADR 2504 (the loss, for a font stating neither a descriptor nor widths), ADR 2686 (the
widths written are the ones the page is drawn by), ADR 2898.
Clauses: ISO 32000-2 §9.6.2.1 (Table 109); ISO 19005-2 section 6.2.11.2 and 6.2.11.5, ISO 19005-4
section 6.2.10.2 and 6.2.10.5.

## Context

Every route that gives an output font its missing `/Widths` writes them from the window's own
`LoadedFont::advance` (`state_laid_out_widths`), so once ADR 2898 changed the window the numbers
followed. What did not follow was the statement: `layout-from-this-viewer` was recorded only on
the route for a font with no descriptor, while a font with a descriptor and no widths — an embedded
program's, a substitute's or a cut program's — was written as though the standard had laid it out.

## Decision

1. **The loss is every font ADR 2898 covers**: wherever the font the window drew reports
   `supplied_widths`, the widths written are this viewer's layout. The embedding route's proofs
   read the restated font with those widths in it, as it is written; without them they read
   `/MissingWidth` and refused a font with a descriptor and no widths. On the embedding route the
   font's own losses carry it; on `fonts/simple-font-states-its-widths` (for now a font drawn
   only in rendering mode 3: an embedded program waits on Q868), which was a `Stated` row, the
   preparation records the fonts and the row becomes `Answer::Loses` for a document holding one
   (a `PerDocument` row of `CONDITIONAL`), as `undecodable-stream` does for its filter row.
2. **The profiles keep A687's split**: `as-if-printed` discards the loss at the new site too, since
   a print carries the layout; `state-every-loss` states it; `as-shown` stops at the default.

## Consequences

A conversion of such a file under `as-shown` now stops where it was written in silence. Where the
no-descriptor route's face states an advance other than the window's for a shown code outside the
resembling metrics, that route refuses as before (`advances_are_the_faces`) rather than write a
face the window did not lay out by.
