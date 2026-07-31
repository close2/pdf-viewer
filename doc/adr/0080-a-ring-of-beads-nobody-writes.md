# ADR 0080 — A ring of beads nobody writes

Status: accepted, 2026-07-31.

## Context

§12.4's row said "page labels, articles and presentations. None implemented, none reported" while
§12.4.2's labels had been `implemented` since the forty-eighth session and §12.4.4's presentations
`partial` since the seventieth. That is ADR 0079's rule arriving one clause later — a parent
understating its children — and the child it was right about was §12.4.3, articles.

An article is a reading order laid over pages that are not consecutive: the catalog's `/Threads`
holds thread dictionaries, each thread's `/F` names its first *bead*, and the beads chain through
`/N` and `/V`. §12.6.4.7's thread action jumps to one of them and was `silent` for the reason its
row gave: the thing it names did not exist here.

## Decision

**Read the threads as data; offer the step; leave the zoom.**

`pdf-model/src/article.rs` reads Table 162's thread — its `/I` information dictionary's `/Title`,
which §12.6.4.7 makes load-bearing — and Table 163's beads from `/F` along `/N`. `Articles::next`
is the clause's own permission ("[i]nteractive PDF processors may provide navigation facilities to
allow the user to follow a thread from one bead to the next") answered as a function. A bead's
`/R` rectangle is read and nothing zooms to it, which is §12.3.2.1's position exactly: this viewer
fits whole pages.

`Action::Thread` is Table 209 whole — `/D` as a reference, an index or a title, `/B` as a
reference or an index — resolved by `ThreadJump::bead_in` against the articles. A `/F` makes it
another file's thread and it is refused by name, `GoToR`'s refusal for `GoToR`'s reason.

## The chain is a ring, and that changes the walk

Every other linked list in this tree ends when an entry is absent. §12.4.3's does not:

> The thread dictionary's F entry shall refer to the first bead in the thread; the beads shall be
> chained together sequentially in a doubly linked list through their N (next) and V (previous)
> entries.

and Table 163 closes it at both ends — `/N` "[i]n the last bead … shall refer to the first bead",
`/V` "[i]n the first bead … shall refer to the last bead". So a reader that stops at "no next"
never stops, and the visited set is not a defence against malformed files but **the termination
condition of a well-formed one**. Only `/N` is followed; `/V` is redundant with it and could only
disagree, which is §12.3.3's outline rule reused.

## The standard states one array's order twice, and the two disagree

A page carries the same beads by another route, and Table 31's NOTE 2 says so: the `/B` entry
"can be created or recreated from the information obtained from the Threads key in the document
catalog dictionary". That makes it checkable — the habit `/Count`, an LZW stream length and a
byte-swapped `indexToLocFormat` all taught here.

What it cannot check is the *order*, because the standard gives two:

- Table 31: "The beads shall be listed in the array in natural reading order."
- §12.4.3: "the page object … shall contain a B entry whose value is an array of indirect
  references to the beads on the page, in drawing order."

Those are not the same thing — a two-column page draws left column then right, and reads the same
way only by accident of layout — and nothing in either clause ranks them. So
`Articles::page_array_agrees` compares the **set**, which is a documented choice and the only
comparison both sentences license, and `beads_on_page` answers from the threads, whose order
§12.4.3 states without contradiction.

Recording it here rather than picking one is principle 5's third case: not that we misread it, not
that they did, but that the specification says two things.

## The corpus says nothing at all, and that is a measurement

`tests/articles.rs` walks every object of all 974 documents. **Not one states an article.** Two
catalogs carry a `/Threads` entry — `issue6961.pdf` an empty array, `noembed-jis7.pdf` a reference
that resolves to null — and no page carries a `/B`. (Twenty-five `/B` keys exist in the corpus and
every one is on something else: an annotation, a sound, twenty-three untyped dictionaries.)

That makes this the cleanest instance of trap 8 the project has: the reader is written from the
clause, its test *is* the clause's EXAMPLE 2, and the corpus test is a ratchet on the corpus
rather than on the code — if a document with a real thread ever arrives, the numbers change and
the test says so.

## Consequences

- `silent` falls 93 → **89**. Four rows close: §12.4, §12.4.1, §12.4.3 (`partial`, for the zoom)
  and §12.6.4.7 (`implemented`).
- §12.6's actions performed rise from five to six, and this one is the first that needed a
  *second* reader built before it could be performed at all.
- No gate moves and none could: an article changes no mark on a page.
- The viewer reads `/Threads` when a thread action is performed rather than when a document opens
  — principle 2's "nothing eager", applied to a list that two documents in a thousand carry and
  neither of those two fills in.
