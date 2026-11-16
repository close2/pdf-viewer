# 2892 — A Private Use ActualText's removal is stated on the appended page

Session 2038, an archive round in the audit worktree, on what ADR 2889 left: `state-every-loss` kept `stop` for
the loss `actual-text-private-use` because its statement page had no sentence for it. Status: accepted and built.
In `pdf-transform`: `config::STATED_GRAPHICS` (the row joins the halftone rows), `statement::TAKEN` (its entry),
`prepare/appended_pages::stated_graphics` (its population, `actual_text::removal`), `tagged::Described::apply`
(the root's `/K` kept by reference); `doc/profiles/state-every-loss.toml`.
Test: `tests/archive/statement.rs::a_private_use_actual_text_is_stated_and_reads_back_against_the_source`.
Context: CLAUDE.md's first near-side construction (A706), ADRs 2562 (the halftone rows' page), 2889.
Clauses: ISO 19005-4 section 6.2.10.8; ISO 32000-2 §14.9.4, §7.9.2.2.

## Decision

The removal has the shape ADR 2562's halftone rows already state: one object losing one entry. So the row is a
fourth entry in the same table, and its lines are the same fields: the requirement and its `asks` sentence, the
loss word with one fixed line, `Object: dictionary <n> <g>`, `Entry removed: ActualText`, `Value in the source:`
the string as the source states it (written by `pdf_syntax::write::object`, so in hexadecimal: `<FEFFF0B7>`), the
pages whose resources name the object, and what was tried before. The value is the producer's bytes, never a
rendering of them, so the page names the characters without composing any; the holder is called a dictionary
because §14.9.4 puts the entry on a structure element or a property list and the page states nothing it did not
read. The line is fixed: "the replacement text, which held a character of the Unicode Private Use Area, is no
longer in this document, so the content it covered is read as its own text" — §14.9.4's reading of content
with no entry. `state-every-loss` answers the row `try = ["append-statement", "discard"]`; a page that cannot be
composed hands the objects to `discard`, as the halftone rows do.

## The root's `/K`, found by veraPDF

The first output this built failed veraPDF 1.30.3 at section 6.2.10.8 and passed this tree's own validator:
appending the statement page's grouping element to a structure tree root whose `/K` named one element by
reference resolved that reference and wrote the source's element inline, `/ActualText` and all, past the removal's
rewrite of the object and beside the object itself, whose kids' `/P` still name it. Table 354 lets `/K` be one
dictionary or an array of them, and an indirect reference is how a dictionary is named, so the array now holds the
reference. The test reads every object's written syntax rather than its top-level keys, and fails on the old
construction. That the validator did not see a direct structure element's entry is left (ADR 2893's round record).

## Not chosen

Writing the value's code points (`U+F0B7`) as the report does: the page's rule (ADR 1014 section 3) is that
every value reads back against the source, and the source's string written in clause 7's syntax does that
without a decoding step a reader would have to trust.
