# ADR 0085 — What a flattened form was

Status: accepted, 2026-07-31.

## Context

Clause 14 had two `silent` rows left, and they were the two smallest kinds of debt a clause can
leave: an attribute family nobody had read, and a worked example nobody had run.

## Decision

**§14.8.5.6's `PrintField` is read**, in `Tree::print_field`. It is the one part of §14.8.5 a
reader can act on rather than a description of how an appearance was produced, and the clause
says why: a non-interactive form

> may have originally contained interactive fields such as text fields and radio buttons but were
> then converted into non-interactive PDF files, or they may have been designed to be printed out
> and filled in manually.

The widget is gone; the marks are all that is left; this attribute is what says the marks were a
check box, whether it was ticked, and what the field was called. A consumer that skipped it would
read a flattened form as a page of unlabelled boxes.

Three details are the clause's rather than a convention:

- **Both spellings of the state entry.** Table 383 lists `Checked, checked` and deprecates the
  lower-case one in PDF 2.0, its NOTE 2 explaining that the old case "did not conform to the same
  conventions used elsewhere in this standard". Both are read, current first — deprecation tells
  a *writer* what to stop doing, the same reading §12.2's `/ViewArea` gets.
- **`off` is applied, not left absent**, because Table 383 states it as the default and an
  unticked box is what a printed form full of them looks like.
- **A `tv` field's value is not in the attribute**: "[t]he text that is the value of the field
  shall be the content of the Form structure element", which ADR 0084's logical order already
  returns.

**§14.7.7's example is run as a document.** The clause is four pages of one PDF rather than a
requirement, so the row closes as a *test*: `tests/logical_structure_example.rs` is the example's
own objects at its own object numbers, asserting that each mechanism it demonstrates is read —
the role map's three mappings, the class map under an element's own override, the parent tree
answering per page with one element named twice, the `/IDTree`, and the paragraph that "spans
pages" resolving to one content item on each, the second through a marked-content reference that
names the other page.

`Tree::element_by_id` is new and exists because of it: the example carries an `/IDTree` and
nothing in this tree had read one. It is `pdf-syntax`'s name tree with §14.7.2's Table 354 entry
on top, which is four lines — the kind of gap only a worked example finds, since no corpus
document writes one.

The fixture also carries the example's `101 1 obj`. That is the only non-zero generation number
in this project's tests, and it is exactly the sort of thing a reader assumes away.

## Consequences

- `silent` falls 66 → **64**, and **clause 14 reaches zero**. Every subclause of the interchange
  clause is now `implemented`, `partial`, `inapplicable` or `out-of-scope`.
- The whole ledger's silence is now two clauses: **62 rows of clause 12's interactive half** and
  **2 of §9.8.3's substitution hints**.
- No gate moves. §14.1 says of this clause that its contents "do not affect the final appearance
  of a document", and that has held for every one of the six sessions spent in it.
