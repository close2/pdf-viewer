# 2380 — XFA's halves are cut where XFA 3.3 cuts static from dynamic, and the static half begins at the datasets packet

Session 1773, a clause round in batch 117. Status: accepted for the reading and the next XFA
round's contract; the scope it serves is RFC 0011's, and the dynamic half waits on Q639. ADR 2381
is unused.

## Context

The owner, 2026-10-26 (`doc/questions/A633`): "I have downloaded XFA. We should start implementing
it one round per batch." The answer asked the first round for an RFC that places a static form's
template behaviours apart from a dynamic form's layout, and for the amendment the layout needs by
name. The brief guessed the halves separate at the template's `layout` and `presence` properties.

## Decision

1. **The cut is XFA 3.3's own.** A subform is static unless its `occur` bounds differ or are
   unbounded, a form with a `subformSet` is dynamic (chapter 9, page 333), and an XFAF form — PDF
   for every page's appearance, XFA for the logic — is held to chapter 7's table of grammar it may
   use only at its defaults (pages 272–273), with Table 29's `/NeedsRendering` false (chapter 2,
   page 72). `layout` is one row of that table; `presence` is on neither side, because its
   automation effects (`inactive`) belong to the static half and its space-taking effect has a
   reader only where a page is laid out. **The table is not a presence test**: all four hybrids in
   the corpora state names it lists (a root subform's `layout="tb"`, a page subform's `w` and `h`),
   which it admits to express the form's logic. A template is XFAF by the flag, the occurrences and
   the subform sets; one that is not is reported as the dynamic half's and never half-run (trap 5).
2. **The conversion stands.** `doc/md/XFA-3_3.md` was made from a PDF with the SHA-256 of the
   owner's `doc/XFA-3_3.pdf`, by `tools/spec-md.py`, which has not changed since; a later round does
   not re-convert it. It is cited by chapter, section title and printed page (the page markers equal
   the folios), and paraphrased, never quoted.
3. **The next XFA round is RFC 0011 section 5's steps 1 and 2**: the resource read into a template
   and its data, and `ViewState::save` writing the `datasets` packet consistent with every `/V` it
   writes. Today a filled and saved hybrid form keeps the producer's values in `datasets`, against
   §K.2's writer rule "The XFA field values shall be consistent with the corresponding V entries of
   the PDF field objects." That step is a writer obligation for an act already in scope — a person
   filling a field and saving it — so it waits on no answer.

## Consequences

- No ledger status moves; Annex K's notes point at RFC 0011.
- `doc/specifications.zip` gains the XFA text only when a test first derives a value from it.
- The dynamic half's first round, if Q639 is accepted, is a measurement of laying out page one of a
  `/NeedsRendering` document before any layout ships.
