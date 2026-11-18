# 2970 — A glyph name is read by the whole Adobe Glyph List algorithm; an unknown component is empty

Session 2074. Status: **accepted and built.** Supersedes, on one point each, the policy `pdf_font::encoding::text_for`
stated in its own comment (a component the list cannot answer for makes the whole name unanswerable) and ADR 2540's
description of `text_for` as declining a `uni` name of several groups. In `pdf-font`: `encoding::text_for`,
`encoding::push_component`. Findings 2038-B15 and 2038-B16. Clauses: ISO 32000-2 §9.10.2; the Adobe Glyph List
Specification, version 2.9, section 2.

## Context

§9.10.2's second method sends a glyph name to "the Adobe Glyph List and Adobe Glyph List for New Fonts". The list's
specification states how a name is read, in four steps: drop everything from the first period; split on underscores;
map each component to a string; concatenate. Step 3 maps a component by the first rule that holds — the ITC Zapf
Dingbats list for a `ZapfDingbats` font, a row of the Adobe Glyph List (a row may hold several scalar values:
`dalethatafpatah` is U+05D3 U+05B2), `uni` followed by groups of four uppercase hexadecimal digits each outside
D800–DFFF, `u` followed by four to six such digits — and otherwise "map it to the empty string".

`text_for` kept one character per component, so a row of several scalar values and a `uni` name of several groups
read as nothing, and the font fell back to the code's byte (the review's witness: `AB` where the page shows A and
the Hebrew sequence). And it refused the whole name when one component was unknown, arguing that half a ligature is
worse than none.

## Decision

`text_for` is the specification's algorithm whole. A component no rule maps contributes the empty string and the
others stand: `A_notaglyph` is `A`, `f_f_notaglyph` is `ff`. The half-ligature argument is a preference about a
file whose producer named a glyph the list does not know; the specification answers the question, and principle 5
puts its answer over ours. A `uni` component whose every group is not a scalar value maps to nothing, rather than to
the groups before the first surrogate (which is where `read_fonts::ps::agl::name_to_chars` stops). An empty
concatenation is `None`, so the caller falls through to §9.10.2's next method as before. The Zapf Dingbats rule
needs the font and stays with the caller that has it (`SymbolicEncoding::character_for`, ADR 0318).

## Consequences

Text extraction and search read multi-scalar rows, multi-group `uni` names and partly-known composed names; the
archive's `/ToUnicode` writer, which asks `text_for`, writes them too. Tests: `pdf-font` `agl_sequences_spec_audit`
(three), `viewer-core` `agl_sequences_spec_audit` (two), and `encoding::tests::a_composed_glyph_name_reads_back_as_its_components`.
