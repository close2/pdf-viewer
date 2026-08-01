# ADR 0091 — The page that is not in the page tree

Status: accepted, 2026-08-01.

## Context

After ADR 0090 the ledger had **one** `silent` row left: §12.7.7's named pages. Its note had said
for many sessions that the clause "[n]eeds the name tree §12.3.2.4 and §12.4.2 also need, which is
this project's third row waiting on one data structure" — and that name tree was built in the
forty-eighth session. The row had been closable for fifty-two sessions and nobody had priced it.

The clause is one paragraph, and it is a rule about a document's own contents:

> A named page that is intended to be visible to a user shall be left in the page tree (see 7.7.3,
> "Page tree" ), and there shall be a reference to it in the appropriate leaf node of the name
> dictionary's Pages tree. If the page is not intended to be displayed by the PDF processor, it
> shall be referenced from the name dictionary's Templates tree instead. Such invisible pages shall
> have an object type of Template rather than Page and shall have no Parent or B entry

## Decision

**Both trees are read, the clause's own rule is checked against the document, and a template page
imported by §12.7.8.3.3 is added to the document being shown.**

The last of those is what makes this a renderer's business rather than a data-interchange
feature. §12.7.7 gives naming a page exactly two purposes and states them itself: an import-data
action adding it, and an ECMAScript action adding it. The second is on `CLAUDE.md`'s closed
exclusion list. The first is §12.7.8.3.3, read in the hundredth session (ADR 0090) — so this
closes both rows at once, and a template page becomes a page a person can turn to.

**Adding a page costs a name lookup and no page content.** That is the shape of the clause worth
carrying: an FDF file's `/Pages` does not carry any page, it carries Table 253 *names*, and every
one of them resolves to a page the target document already holds. The whole of "importing a
template page" is therefore a lookup in a tree this reader already walks.

**A template page is built without inheritance.** §7.7.3.4's attribute inheritance runs up
`/Parent` and this clause forbids a template one, so a template states everything it needs.
`Pages::detached` is `Pages::get`'s own assembly with an empty ancestry — the same defaults, the
same media-box intersections, the same `/ViewArea` treatment — rather than a second page builder
that could drift.

**Where the page goes is this program's choice, and it is written down.** §12.7.8.3.3 says a
template page is added to the document and states no position. After the document's own pages is
the only order that leaves every existing page index meaning what it meant, which matters because
a destination, an outline entry and an article bead all name pages by index or by object.

## Two refusals and one entry nobody can implement

Table 253's `/F` puts the template in another file — `GoToR`'s refusal exactly — and a `/TRef`
naming no page in either tree is a file asking for something the document does not contain. Both
are named on `Imported::refused` and printed.

Table 252's `/Rename` is the third, and it is unusual: the clause implements the refusal itself.
The flag decides whether a template's fields are renamed on a name conflict, and the standard says
in as many words that "the `Rename` flag does not define a renaming algorithm", then suggests one
a processor "might" use. What renaming decides is which fully qualified name a template's field
answers to afterwards; this program does not merge field trees, so a template page's widgets draw
from the template page's own fields and no name collides. Read, and nothing for it to change.

## `name_pairs` resolved away the thing this clause needs

`pdf_syntax::tree::name_pairs` resolved each leaf's value, which is what its four existing callers
want. A named page is identified by its **object** — it is what `Pages::index_of` compares, what a
destination carries, and what `ViewState` has to hold — so resolving the leaf threw the identity
away before anybody could ask for it. `collect` now hands `push` the value *as stated* and the two
existing functions resolve in their own closures, with `name_entries` beside them for the caller
that wants the reference. One walk, one cycle guard, three views of it.

## The measurement corrected the comment before it was committed

This module's doc comment was about to say "no corpus document has one", which is exactly the
shape of claim trap 8 exists for. Running `NamedPages::read` over all 964 openable corpus documents
took one throwaway test and says: **one document names a page** — `issue19389.pdf`, a single entry
in the `/Pages` tree — and **none** states a `/Templates` tree. That one document agrees with all
four of the clause's invariants.

It changes nothing about the code and it changes the sentence. The general form is already in
Habits — *a "not implemented" count of zero can mean "nothing reports it"* — and this is its
cheaper cousin: a corpus claim costs one run of code that already exists.

## Consequences

**The conformance ledger reaches zero `silent` rows**, for the first time since it was built in
the ninth session. All 823 subclauses of the eight technical clauses are `implemented` (358),
`partial` (230), `reported` (53), `inapplicable` (88), `out-of-scope` (87) or `writer-side` (7).
The status exists and no row carries it.

What that does *not* mean is that the standard is implemented — `partial` and `reported` are 283
rows between them, each naming what it owes. What it means is narrower and is the thing the ledger
was built to say: **there is no requirement in those eight clauses that this program fails without
saying so.** The specification track's map is now entirely the 53 `reported` rows and the notes on
the 230 `partial` ones.

No gate moved: 89 corpus documents incomplete, 65 contradicted pages, 97.8% text readback. Expected
— one corpus document names a page and nothing imports data into it.
