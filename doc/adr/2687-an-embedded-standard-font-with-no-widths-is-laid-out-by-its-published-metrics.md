# 2687 — An embedded standard font with no widths is laid out by its published metrics

Session 1933, a single archive round.
Status: accepted and built. In `pdf-font`: `loading/opening.rs` (`published_metrics`) and
`metrics.rs` (`SimpleMetrics::published`, read by `simple_widths`). Pinned by
`type1_widths::a_standard_name_stating_a_descriptor_and_no_widths_is_given_the_widths_it_is_laid_out_by`,
whose source reads Courier's 600 units where it read 0 before.
Context: ADR 2686 (the row and remedy that write these widths), the standard-metrics fallback
`simple_widths` already applied to a substitute.
Clauses: ISO 32000-2 §9.6.2.1 (Table 109 and the paragraph beneath it), §9.6.2.2, §9.2.4,
§9.8.1 (Table 120's `/MissingWidth`); ISO 32000-1 Table 111.

## Context

A simple font's advances came from `/Widths`; failing that, for a substitute only, from the
standard's published metrics of the face asked for, then from the substitute's program; and
anything unanswered took `/MissingWidth`, 0 by default. An embedded program under one of the
fourteen names whose dictionary states a descriptor and no widths reached none of those, so
every glyph advanced by 0. `issue11150_reduced` draws `[(q)521(q)(q)521(q)…] TJ` under such a
`/Symbol`, a test whose `521` is Symbol's published width for `theta`; this reader stacked every
glyph on the first.

## Decision

The standard's paragraph beneath Table 109 decides it: "For compatibility reasons PDF processors
shall provide glyph widths and font descriptor data for those standard fonts for use in
processing PDF files when the entries are absent." It speaks of the fourteen of §9.6.2.2 and of
the widths being absent, and says nothing that turns on a descriptor or a program being present.
So a `Type1` dictionary whose `/BaseFont` is one of the fourteen names, spelled exactly, and
that states no `/Widths`, is laid out by that face's published metrics, by glyph name; a glyph
those metrics do not name keeps `/MissingWidth`. ISO 32000-1 Table 111 makes such a dictionary
malformed and says nothing of how to read it, and Table 109's `/MissingWidth` for every code
would put every glyph at one advance, which no producer of a text-showing operator means. The
program's own advances were the other candidate; they are not chosen because the sentence names
the standard font's widths, and §9.2.4 makes the dictionary's statement, here the standard's,
the one that positions. Where the program disagrees, ISO 19005's 6.2.11.5 restatement moves the
program, never the page (ADR 0947).

A name with a subset prefix, a TrueType dictionary under a standard name and a non-standard name
are not the fourteen and keep `/MissingWidth`.
