# 2492 — The scope ratifications live in a document of their own, and `CLAUDE.md` keeps each rule and its reason

Session 1829, 2026-10-27. Status: **implemented**. ADR 2493 was assigned to this round and is unused.
Context: `CLAUDE.md` is in the context of every request every agent makes. Its "What *done* means"
section carried, beside each exclusion and each amendment to the authoring line, the date it was
ratified, the question and the owner's answer, the ADR and RFC pointers, the trigger that reopens an
exclusion, and the construction behind each amendment. Review 1814 measured the file's weight and ADR
2462 section 3 proposed moving that detail; the owner agreed, `doc/questions/Q680` item 1 and
`A680` ("Q680 agree"). Cites no clause and moves no ledger row.

## Decision

1. The detail is `doc/scope-ratifications.md`, named in the first table of `CLAUDE.md` ("moved out of
   this file", beside `doc/stack.md` and `doc/environment.md`) and in a row of `doc/HANDOVER.md`'s index.
   Each entry is the full ratification of one exclusion or amendment, in the words it was ratified in.
2. `CLAUDE.md` keeps, for each exclusion and each of the three amendments, the rule and the reason in a
   sentence or two; the closed list, the boundary line ("does the operation invent marks?"), the
   statement that a user's edit is written back by an incremental update, the immutability of
   `pdf_syntax::Document`, and "Revisit an exclusion by argument, never by attrition" stay whole.
   Principles 1 to 5 and the in-scope list are untouched: no rule is reworded, softened or added.
3. The word "fifth" is dropped from the XFA dynamic-half paragraph, as the owner asked on 2026-10-27:
   "the fifth near-side construction" reads "the near-side construction", in `CLAUDE.md` and in the
   document.

## Why this arrangement

A paragraph that is a rule and its reason is what every request needs; a date, a question number and
an ADR pointer are what a round needs once, when it touches scope. The stack table and the working
agreements already follow that pattern, and a round opens `doc/stack.md` as it would open this
document (ADR 1023 applies the same test to comments: what a reader needs now stays where it is read). The cost, which Q680 stated against the move, is a paragraph a
round may not open: the sentence left in `CLAUDE.md` names the document, and the table row says when
to open it.

## Proof that nothing was lost

I worked in `scratchpad/r1829/`. The sentence diff, a script run as `python3 -I -B`, splits the old `CLAUDE.md` into sentences
and looks each up, whitespace and list markers normalised, in the new `CLAUDE.md` and then in the new
document. Of 211 old sentences, 180 stand unchanged in `CLAUDE.md`, 29 stand unchanged in the document,
and 2 are in neither: the lead-in "The exclusions, closed, each with its reason:", which now carries a
parenthesis naming the document, and the XFA sentence that held "fifth". Both are in `CLAUDE.md` or the
document with that one change. `CLAUDE.md` gained 12 sentences or table rows, each a one-sentence form
of a paragraph the document holds whole, and the document 14 sentences or headings of introduction.
The file went from 31 834 to 27 002 bytes.

## Consequences

A change to a ratification is made in the document; a change to a rule is the owner's and is made in
`CLAUDE.md`. `tools/conformance` treats `doc/*.md` as an instruction document, so the `§` rule holds
over the new file (`cargo test -p conformance --test documents`).
