# 2917 — A Type 3 width written as its font states it is stated on the appended page

Session 2051, a PDF/A round in the audit worktree, on what ADR 2907 left: `state-every-loss` stated no
line for the loss `type3-width-as-drawn` and kept ADR 2847's photograph. Status: accepted and built.
In `pdf-transform`: `config::STATED_TYPE3_WIDTHS` (the row joins `statement_hands_on`),
`statement::type3_widths_entry` (its entry and fixed line), `prepare/appended_pages`
(`type3_widths_may_be_stated`, `stated_type3_widths`, the page's ninth list), `report::Listed`
(`type3_width_records`); `doc/profiles/state-every-loss.toml`.
Tests: `tests/archive/type3_width_as_drawn.rs::state_every_loss_writes_each_width_and_states_it_on_its_appended_page`;
`tests/archive/photographed.rs`'s Type 3 photograph now under a plan withdrawing the one loss.
Context: CLAUDE.md's first near-side construction, ADRs 2638 (the operator row's page, the shape
taken), 2892, 2907. Clauses: ISO 19005-4 section 6.2.10.5; ISO 32000-2 §9.6.4 (Table 111), §9.2.4.

## The question, and the answer the profile's text gives

`state-every-loss` promises, in its own first line, to "keep what can be kept, and state every loss
taken on an appended page", and it is `no-data-loss` with `append-statement` where that profile
stops. The near-side construction lets such a page "state what an archival conversion removed, in
facts computed from the file and a fixed vocabulary naming the requirement and the loss". The
width is one operand of one procedure, and every fact about its removal is in the rewrite's record:
the font, the glyph, the procedure (and the copy where one was made), the operator, the operand as
the producer's bytes spell it, and the width written. That is the shape ADR 2638 already states
for the operator row's runs, a number written in a content stream where the producer's characters
stood. A photograph, by contrast, loses the page's text, which this profile exists to keep. So the
profile grants the loss and states it: `try = ["append-statement", "discard"]`.

## Decision

1. **One entry per width written**, in the report's own records, so the page names exactly what
   the report and `xmpMM:History` name: the requirement with its `asks` sentence, the loss word and
   one fixed line, `Font: dictionary <n> <g>` (or the words for a font written directly),
   `Glyph: /<name>`, `Procedure: stream <n> <g>`, `Copy written for this font: stream <n> <g>`
   where the copy is not the procedure, `Width as the producer wrote it: <operand> <operator>` and
   `Width written: <width>`. Numbers are the source's; a byte outside printable ASCII is `\xHH`.
2. **The fixed line** is Table 111's and §9.2.4's: the procedure stated a width other than the
   font's, the font's width — the one the page is laid out by — is written in its place, and the
   width the procedure stated is gone from it.
3. **A page that cannot be composed hands the widths to `discard`**, which writes them all the
   same, and the report says why, as at every row of `statement_hands_on`.
4. The photograph remains for a caller whose plan grants `page-as-image` and withholds this loss;
   no shipped profile now does.

## Not chosen

Counting procedures per font on one line: a reader checking the page against the file needs each
procedure's operand, and the history already lists them one by one.
