# 3110 — A Type 3 code outside FirstChar to LastChar is given a procedure of width zero

Session 2102, an archive round in the `lane3` worktree, on `doc/pdfa-blocking-issues.md`'s R03a
documents `Type3Test` and `Type3WordSpacing`. Status: accepted and built. In `pdf-transform`:
`archive/type3_glyphs.rs` (`given_glyphs` reads a code with no `/Widths` entry at Table 110's 0).
Test: `crates/pdf-transform/tests/archive/type3_glyphs.rs`
`a_code_outside_first_char_to_last_char_is_given_a_procedure_of_width_zero`. Context: ADR 2918 (the
empty procedure, whose "a code with no width" refusal this narrows), ADR 0299 (the corpus file and
its word spacing), ADR 2652 (veraPDF's 6.2.11.4.1-2 on both files, recorded as undecided).
Clauses: ISO 32000-2 §9.6.4 and its Table 110, §9.3.3; ISO 19005-2 sections 6.2.11.4.1 and
6.2.11.5; ISO 19005-4 sections 6.2.10.4.1 and 6.2.10.5.

## Context

Both files show the single-byte code 32 in a Type 3 font whose `/FirstChar` is 97 and whose
encoding names nothing for 32. ADR 2918 gives such a code a `w 0 d0` procedure at its `/Widths`
entry, and refused a code with none: `given_glyphs` read the entry with `?`. Table 110 states
the width of every such code: "For character codes outside the range FirstChar to LastChar, the
width shall be 0". This reader lays the code out by that 0 (`pdf_model::type3::Type3Font::advance`
quotes the same sentence), plus the word spacing §9.3.3 applies to the single-byte code 32.

## Decision

A shown code that has no `/Widths` entry is given the procedure `0 0 d0`, under the fresh name ADR
2918 gives an unnamed code. Nothing else changes: an entry that is present and is not a number
still refuses, and the font's `/Widths`, `/FirstChar` and `/LastChar` are left as written, so ISO
19005-4 section 6.2.10.5 compares the procedure's 0 with the 0 Table 110 gives the dictionary.
The word spacing is the graphics state's and moves the next glyph on both paths.

## Consequences

`Type3Test` and `Type3WordSpacing` convert at 2b and 4 under `as-shown` and `as-if-printed`;
veraPDF 1.30.3 passes all eight outputs, which closes ADR 2652's 6.2.11.4.1-2 disagreement for
these two files, and page 1 at 300 dpi of each is byte-identical to its source's. The test holds
the text layer's positions and text to the source's under `3 Tw`.
