# 2898 — A font outside the fourteen that states no widths is laid out by the best statement it makes

Session 2041, a font round in the audit worktree, on the owner's answer `doc/questions/A859`.
Status: accepted; built for the substitute routes, the embedded and metrics-only routes held for
`doc/questions/Q868`. In `pdf-font`: `metrics.rs`'s `simple_widths`, `resembling_widths` and
`SuppliedWidths`; `loading/opening.rs`'s `within_the_fourteen`; `LoadedFont::supplied_widths`. In
`pdf-model`: `content/font.rs`'s `note_supplied_widths`.
Tests: `crates/pdf-font/src/metrics.rs`'s `substituted_width_tests` (two new or rewritten),
`crates/pdf-model/tests/unstated_widths.rs` (all three).
Context: ADR 0007 (the fourteen's published metrics), ADR 2687 (and under an embedded program),
ADR 2870 (a program present and unreadable is drawn from a substitute), ADR 2899 (the archive
follows this).
Clauses: ISO 32000-2 §9.6.2.1 (Table 109), §9.6.2.2, §9.8.1 (Table 120), §9.2.4.

## Context

Table 109 makes `/Widths` "Required; optional in PDF 1.0-1.7 for the standard 14 fonts". A font
outside §9.6.2.2's fourteen that states none is missing an entry the standard requires, and the
standard describes valid files: it states no layout for such a font. The tree read it three ways
by route (Q859): a substitute by the resembling standard font's metrics and then its own face's
advances, an embedded program and a metrics-only font by Table 120's `/MissingWidth` for every
code, which drew `issue5470`'s line as one pile of glyphs.

## Decision

1. **The layout is a choice made on the file's defect, not a clause obeyed**, and the owner chose
   it (A859, option (b)). For a simple font outside the fourteen whose `/Widths` states nothing,
   each code takes, in this fixed order, every route alike: the embedded program's own advance for
   a glyph that code reaches and that is not `.notdef` (Table 109's "consistent with the actual
   widths given in the font program"); else the published metric of the standard font its name
   and descriptor resemble (`substitute::Request::derive`, the request a substitute is chosen
   by); else Table 120's `/MissingWidth`, stated or 0.
2. **Built for the routes that draw a substitute**: a font naming no program (ADR 0007), one
   whose program is present and unreadable (ADR 2870), and one stating no descriptor (A687). They
   have no program of the file's to ask, so the published metrics come first and then
   `/MissingWidth`; the substitute face's own advances are no statement of the file's and are no
   step, where they used to answer a code the metrics do not name.
   **Held for Q868**: an embedded program that parses, and a metrics-only font (ADR 1094), keep
   Table 120 for every code, because the owner's audit pins exactly that
   (`loading.rs::the_program_states_an_advance_of_its_own`,
   `refused_font_metrics.rs::a_refused_program_falls_back_to_the_stated_missing_width` and
   `::a_refused_program_stating_no_width_advances_by_nothing`), and round 2000's rule makes a
   change contradicting an audit test a question rather than an edit.
3. **The fourteen are unchanged**: Table 109 admits them without the array, so the published
   metrics come first and the program drawn answers what they do not name (`issue4304`'s
   `.notdef`); nothing is reported.
4. **It is said out loud.** A page that loads such a font notes `Unsupported::Font` with how many
   codes the metrics answered and which standard font resembled, and is incomplete: what it shows
   is not laid out as its producer specified, because the producer specified nothing (trap 5). A
   stand-in this reader invents for a construction (ADR 0112) is no font of the file's and is not
   reported here; its construction already says what stood in.

## Consequences

`issue5470`'s line reads as a line and its page says why its layout is this reader's. Pages whose
fonts take this route leave the oracle's judged set, and a substituted line showing glyphs outside
the standard set closes up where the face's advances used to open it; the round's record lists the
pages that moved.
