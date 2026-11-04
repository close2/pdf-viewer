# 2686 — A simple font states its widths, and the checker fails one that does not

Session 1933, a single archive round.
Status: accepted and built. In `pdf-archive`: `table/fonts.rs` (the row
`fonts/simple-font-states-its-widths`, `simple_font_states_its_widths`) and `coverage.rs` (the
row named under the first sentence of 6.2.11.2 and of 6.2.10.2). In `pdf-transform`:
`archive/font_widths.rs` (`laid_out`, `WidthArrays::stated`), `archive/rewrite.rs`
(`Rewrite::FontWidthsStated`), `archive/decision/fonts.rs` (its remedy), `archive/prepare.rs`;
`archive/fonts.rs` loses the refusal ADR 2685 decision 5 added. Tests:
`table::fonts::tests::a_simple_font_states_its_widths_unless_a_standard_one_states_none_of_the_four`
and `type1_widths::a_standard_name_stating_a_descriptor_and_no_widths_is_given_the_widths_it_is_laid_out_by`.
Context: ADR 2685 decision 5 (the refusal this replaces), ADR 2146 (the substitution route
already states laid-out widths for one of the fourteen), ADR 2178 (the sibling row, a
`/Widths` of the wrong length), ADR 2433 (the used-font population), ADR 2687 (the widths
this reader lays such a font out by).
Clauses: ISO 19005-2 section 6.2.11.2, ISO 19005-4 section 6.2.10.2; ISO 32000-1 Table 111,
§9.6.2.2, §9.6.2.3, §9.6.3; ISO 32000-2 §9.6.2.1 (Table 109), §9.2.4, §7.3.9.

## Context

Both parts hold every font a file uses to the base standard's §9.6 and §9.7, and the two
editions differ on the four entries. ISO 32000-1 Table 111 marks `/FirstChar`, `/LastChar`,
`/Widths` and `/FontDescriptor` each required "except for the standard 14 fonts", and for those
"shall either all be present or all be absent". ISO 32000-2 Table 109 marks each "Required;
optional in PDF 1.0-1.7 for the standard 14 fonts", and a part 4 file is a PDF 2.0 file, so
there the exemption is gone. Round 1932 found `issue11150_reduced`'s `Symbol`, which states a
descriptor and no widths, failed by veraPDF at 6.2.11.2-4, -5 and -6 and passed by this tree's
checker, which had no row for it. A checker that passes a non-conforming output is the worst
defect a validator can have; the converter refused the document instead, by name.

The premise, swept with `cargo run -p pdf-archive --example one -- <file> <target>` over the
census sources and every output rounds 1931 and 1932 kept: the first reading of the row, over every font dictionary the file holds, failed 23 sources at 2b
and 512 at 4, and 1258 outputs this tree had written as conforming. Of those outputs, 1250 were
part 4's, every one a `/Helvetica`, `/ZapfDingbats` or similar named only in an interactive
form's `/DR`, which no content stream draws and veraPDF passes. The clause binds the fonts
*used* in the file, so the population was wrong, not the outputs.

## Decision

1. **The row is `fonts/simple-font-states-its-widths`, at 6.2.11.2 and 6.2.10.2.** A `Type1`,
   `MMType1` or `TrueType` dictionary a text-showing operator ran with, in any rendering mode,
   states all four entries; at part 2 a `Type1` dictionary whose `/BaseFont` is one of
   §9.6.2.2's fourteen names, spelled exactly, may leave out all four and only all four. §9.6.3
   gives a TrueType dictionary Table 111's entries, and the fourteen are Type 1 fonts, so a
   `TrueType` `/Arial` is not exempt. A `null` value is §7.3.9's absence. The finding names
   the font and the table. A Type 3 font is Table 112's with its own descriptor condition and is
   not judged here.
2. **The remedy writes the three widths entries as the advances this reader lays the glyphs by**
   (`Rewrite::FontWidthsStated`), for a font that states a descriptor. It is the substitution
   route's `state_laid_out_widths` (ADR 2146), read off the source as loaded: for one of the
   fourteen names the published metrics (ADR 2687), otherwise the descriptor's `/MissingWidth`,
   which Table 109 sends every code outside a stated range to. A font with no descriptor has no
   program and is the embedding route's, which writes all four with the face it embeds. A font
   written directly inside another object, or one this reader cannot load, is refused by name.
3. **It is a stated interpretation, not a loss and not "none visible".** No mark moves in this
   reader, and nothing of the file is dropped, so no loss is owed. But the source is a dictionary
   the base standard does not admit, and another reader may have laid it out by the program's own
   advances or by `/MissingWidth`; the output states one reading so every reader shares it, and
   the report says so in the row's sentence.
4. **ADR 2685 decision 5's refusal is removed.** Where the program's advances also disagree,
   6.2.11.5's restatement holds the program to the same advances this row writes, so the two
   statements agree once both are written.

## What moves

The census and veraPDF results are the round's record (`doc/history/1933-*.md`).
