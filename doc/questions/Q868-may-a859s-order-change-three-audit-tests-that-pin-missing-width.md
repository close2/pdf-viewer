# Q868 — May A859's order change three audit tests that pin `/MissingWidth`?

Source: round 2041, building `doc/questions/A859` (ADR 2898). Nothing waits on the answer: the substitute routes
are built, and the two routes below keep what they draw today.

## What was found

A859 orders the advances of a simple font outside §9.6.2.2's fourteen that states no `/Widths`, "across all
simple-font routes": (1) an embedded program's own advances, (2) the resembling standard font's published
metrics, (3) Table 120's `/MissingWidth`. Two routes cannot take that order without changing tests the owner's
audit (`doc/reviews/1983-specification-audit/features.json`) lists, and round 2000's rule makes such a change a
question rather than an edit:

- **An embedded program that parses.** `crates/pdf-font/src/loading.rs::the_program_states_an_advance_of_its_own`
  asserts that a symbolic TrueType font stating no `/Widths` advances code 65 by 0 (`/MissingWidth`'s default)
  while its `hmtx` states 500 per mille. Step (1) makes that advance 0.5 em.
- **A metrics-only font** (ADR 1094, a refusal no substitute recovers).
  `crates/pdf-model/tests/refused_font_metrics.rs::a_refused_program_falls_back_to_the_stated_missing_width` and
  `::a_refused_program_stating_no_width_advances_by_nothing` assert 800 and 0 for `A` of a `/NotAFont` TrueType
  font with a descriptor and no `/Widths`. Step (2) makes both 667 (Helvetica's, the flags resembling it).

The tests' other assertions (the program's own advance is a second number; a stated width and a refused program
still place the pen) hold under either answer.

## The question

May those three assertions change to A859's order?

- **(a)** Yes: the program's `hmtx` answers the first test's code (0.5), and Helvetica's 667 answers the other two;
  each test's comment cites ADR 2898 and A859, and the report each page now owes is pinned beside them.
- **(b)** No: an embedded program and a metrics-only font keep Table 120 for every code, and A859 holds only where
  a substitute is drawn. Their pages keep drawing a line that states no widths as one pile of glyphs.

## Recommendation

**(a)**. The tests were written to pin Table 120 against an invented half-em (ADR 0039), and A859 is the owner's
later answer for exactly the file these fixtures describe; under (b) the window gives one defect two layouts by
route, which is what Q859 asked to end.

## Meanwhile

The substitute routes take A859's order and report it (ADR 2898); the archive writes their widths under
`layout-from-this-viewer` (ADR 2899). An embedded program and a metrics-only font that state no `/Widths` advance
by `/MissingWidth` for every code, unreported, as before.
