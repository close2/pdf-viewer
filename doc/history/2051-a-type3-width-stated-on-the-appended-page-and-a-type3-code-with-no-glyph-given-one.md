# 2051 — A Type 3 width stated on the appended page, and a Type 3 code with no glyph given one

Session 2051, a PDF/A round in the audit worktree. Row 9.6.4; ADRs 2917 and 2918; no question.

**The slice.** The two items round 2045 left, in order: `state-every-loss` stated no line for
`type3-width-as-drawn`, and our validator passed a Type 3 code veraPDF failed under the embedding clause.

**The premise.** Both unbuilt and effort only. The profile's own sentence ("state every loss taken on an
appended page") and CLAUDE.md's statement-page construction decide the first; ISO 32000-2 §9.6.4 b) defines
what is painted for the second, so nothing waits on the owner.

**Built.** `config::STATED_TYPE3_WIDTHS` joins `statement_hands_on`; `statement::type3_widths_entry` lists each
width written with its font, glyph, procedure, copy, the producer's operand and the width, under one fixed line;
`state-every-loss` answers `try = ["append-statement", "discard"]`; `Listed` counts the records. The validator's
`fonts/embedded-programs-define-every-glyph-shown` asks a Type 3 font for each shown code: named by the encoding,
and the name a `/CharProcs` stream (part 2 section 6.2.11.4.1, part 4 section 6.2.10.4.1). `archive/type3_glyphs.rs`
gives such a code a procedure `w 0 d0` at its `/Widths` entry, a named code keeping its name and an unnamed one
given `empty` in `/Differences`: `Rewrite::Type3GlyphDescribed`, `Mechanical`, the row's first answer before
ADR 2543's glyph.

**Tests.** pdf-archive: one validator test (no name; a name with no procedure). pdf-transform: the statement test
in `type3_width_as_drawn.rs`, two in `type3_glyphs.rs` (both parts, `as-shown` and `refuse-any-loss`, layout and
text unchanged); the Type 3 photograph test now withdraws the loss from its plan. veraPDF 1.30.3 passes all nine
fixture outputs and fails both sources at 6.2.11.4.1-2 and 6.2.10.4.1-2 (and 6.2.11.5-1, which the output also
meets). **Plant**: the validator's Type 3 branch dropped fails its test and both remedy tests; the statement's
widths withheld fails the statement test.

**Gates.** Over `pdf-archive`, `pdf-transform`: fmt exit 0; clippy `-D warnings --all-targets` exit 101 on a
doc quote's `CharProcs`, then on two functions past 100 lines and three single-pattern matches, then exit 0;
nextest exit 100 on `editions` (a §9.6.5.3 citation ISO 32000-1 numbers otherwise), then 1565 of 1565 passed;
rustdoc `-D warnings --document-private-items` exit 0; `cd fuzz && cargo check --bins` exit 0 (`Listed` gained
a field). Conformance after `git add -N`: exit 0. Tier 2 (`corpus`, `archive_corpus`) is the train's.

**Left of the rows.** 9.6.4 stays `implemented`. A font a Type 3 rewrite already writes (the `.notdef` rename,
the width restatement) keeps this row's refusal by name; a document mixing a Type 3 font and a program font under
the row takes one answer for both, as the `.notdef` row does.
