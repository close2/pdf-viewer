# 2871 — A cut program the window draws from a substitute is written as that substitute

Session 2027, the archive half of ADR 2870, on the owner's answer `doc/questions/A837`.
Status: accepted and built. In `pdf-transform`: `archive/fonts.rs` (`CutProgram::drawn_as_nothing`
and `describe`, `cut_program`'s `drawn_as_nothing`, the emptying gated on it, `program_dropped`
through `pdf_font::without_program`), the loss `unreadable-program`'s documentation in
`decision.rs`, `config.rs` and `statement/fonts.rs`; `archive/fonts/machine_cid.rs`'s `embed`
reading `drawn_by_the_window`; `doc/profiles/as-shown.toml` and
`as-if-printed.toml`'s comments.
Tests: `crates/pdf-transform/tests/archive/cut_program.rs` (all nine).
Context: ADR 2636 (a cut program takes the unembedded route), ADR 2830 (a font the window drew as
nothing is embedded drawing nothing), ADR 2870 (the window draws a substitute).
Clauses: ISO 32000-2 §9.5, §9.7.4.2, §9.8.1, §9.9; ISO 19005-2 section 6.2.11.4.1, ISO 19005-4
section 6.2.10.4.1.

## Context

ADR 2830 made the archive follow the window: where `cut_program` found a rendered font's program
cut or unparsed and `LoadedFont::load` refused it, the face ADR 2636's route wrote had every shown
glyph emptied, under the loss `unreadable-program`, because the window drew nothing. Its decision 1
said that should the window come to draw a substitute, `cut_program` changes with it. ADR 2870 is
that change.

## Decision

1. **`cut_program` asks the window's question.** `CutProgram::drawn_as_nothing` is
   `LoadedFont::substitute_for_unreadable` over `LoadedFont::load`'s refusal answering `None` —
   the same function the window calls, so the two cannot disagree about a font.
2. **A font the window draws from a substitute takes ADR 2636's route whole**: a simple font is read
   through `pdf_font::without_program`'s copy (the window's own) and given the face that route
   chooses and proves, unemptied, and may carry the machine's face it was drawn from where the
   profile withholds `machine-face`; a composite font goes through `composite::embed` as a font
   naming no program does, and where the profile withholds `machine-face` its machine-face route
   (ADR 2773) takes the face from the font the window draws (`drawn_by_the_window`), so its proof
   holds the output to the substitute's outlines and advances. The losses are that route's own (`machine-face`, `substitute-stretch`,
   `glyphs-from-to-unicode`, `layout-from-this-viewer`, `text-as-image`), authorised or refused at
   their site as for an unembedded font. The report, the JSON `cut_program` field and
   `xmpMM:History` still name the program dropped, now saying the reader drew a substitute.
3. **`unreadable-program` is narrowed, not retired.** It stays the loss of a font the window draws
   from its widths alone: a composite font §9.7.4.2 leaves no route to a substitute, and a simple
   font whose program parsed and whose encoding reaches none of its glyphs (which
   `pdf_archive::program_unparsed` counts unparsed and ADR 2870 decision 1 does not substitute).
   Both still occur, so the loss word, its per-loss key and the profiles' grants stand.

## Consequences

The witnesses' archive outputs draw their text where they drew the window's nothing; the round's
record has which conversions it ran. `issue11651` converts under `as-shown` with the machine's face,
its page pixel for pixel the window's. Under `as-if-printed` the shipped face is written under the
face losses, so the text is the window's text in the shipped face's shapes and fitting. R09's
census counts predate this and are owed a re-run.
