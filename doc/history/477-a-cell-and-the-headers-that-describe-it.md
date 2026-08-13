# 477 — A cell, and the headers that describe it

**Finding.** A screen reader announces a table cell's headers before the cell, and this program
named none: a data cell reached the bus as *23* where the document says *Monday, Sydney: 23*.
§14.8.4.8.3 states the answer twice and the clause chooses between them — Table 384's `/Headers`
array where the producer wrote one, expanded by the entry's own recursion, and an algorithm for
every cell that states none. **Implementing only the array would have answered for 279 cells of the
17 431 that have an answer**: over 1251 files and 21 883 table cells, 281 state the entry and
**17 152 of the 17 431 cells that end with a header get it from the search**. Both routes are read
now, in the clause's order — the row's headers, then the column's, most specific first — and both
were read back off a real AT-SPI bus: `bug2014080.pdf` for the search, `pdfjs_wikipedia.pdf`, whose
ten cells resolve through an `/IDTree`, for the array.

**Date.** 2026-08-13.
**ADR.** [0312](../adr/0312-a-cell-and-the-headers-that-describe-it.md).
**Touched.** `crates/pdf-model/src/structure.rs` (`CellFacts`, `Tree::cell_headers`,
`Tree::cell_facts`, `TableColumns`, `TableStack` rebuilt to keep what it places, `headers`,
`resolve`, `search`, `covering`, four tests),
`crates/pdf-model/examples/cell_header_census.rs` (new),
`crates/pdf-model/examples/table_header_census.rs` (the new `enter`),
`crates/viewer-core/src/accessibility.rs` (`AccessibilityNode::headers`, the second pass, the
remap in `prune`), `crates/viewer-core/examples/accessibility_cost.rs` (what the answer holds),
`crates/viewer-core/tests/headless.rs` (a test on the existing table fixture),
`crates/viewer-confined/src/protocol{.rs,/panels.rs}` (the headers on the pipe, checked the way the
parent link is, and a `TD` carrying one in the round trip),
`crates/viewer-accessibility/src/tree.rs` (`headers`, `spoken_headers`, the composed description)
and `tests/tree.rs`, `doc/conformance/ledger.toml` (§14.8.4.8.3, §14.8.5.7),
`doc/todo/31-accessibility-host.md`, `doc/todo/README.md`, `doc/verify.md`, `doc/adr/0312-*`,
this file.

## What the bus found that the tests could not

The first end-to-end run showed **every cell in `bug2014080.pdf` with no headers at all** while
twelve unit tests passed. That document puts each cell's words in a `P` inside the cell, so every
`TH` in it has an empty `AccessibilityNode::name` — correct, because that field is deliberately the
element's own text and not its subtree's. A cell *named as a header* is the one place the subtree is
what is wanted, and `tree::spoken_headers` is what was missing. Third finding of this shape off the
same instrument (ADRs 0214, 0301).

## Two things the round decided rather than built

`accesskit::Node::set_labelled_by` is the relation this is, and `doc/todo/31` said the AT-SPI
adapter exposes it. It does not — `relation_set` builds `ControllerFor` and nothing else — and it
is worse than inert, because `accesskit_consumer::Node::label` falls back to the labelled-by nodes'
text, so an empty cell would have been announced as its own headers. The headers go into the node's
**description** instead, which is the channel ADR 0300 already used and is a choice about a
platform rather than a reading of the standard.

And Table 384's `/Short`, whose EXAMPLE is precisely this feature, is **stated by 0 of the corpus's
6197 `TH`** — written into `doc/todo/31` with the count beside it rather than built.

## What the measurement found that it was not taken for

ISO 32000-2's **page 400 answers with no nodes at all**. `viewer_core::accessibility::walk` stops
at `MAX_NODES` = 8192 elements of the whole document's tree and prunes to the page afterwards, so a
screen reader on a thousand-page tagged document hears nothing past the first few pages and nothing
says so. `doc/todo/31` carries it; not taken.

And a stale claim retired: `Tree::element_by_id`'s doc comment said no tagged corpus document
states an `/IDTree`. Twelve of the 89 do, and 22 of the wider corpus's 151 — which is where all 475
of Table 384's identifiers are found.

## The cost, and the instrument it needed

Five other rounds were building on this machine and the same release binary read 56 ms and 151 ms
for the same work, so the stopwatch could not see a percentage. `valgrind --tool=callgrind` over
`viewer-core --example accessibility_cost` is load-independent and exact, and the query separates
from the open by running it 1 and 11 times: **+1.17%** on ISO 32000-2 and **+4.5%** on
`Tagged-PDF-Best-Practice-Guide.pdf`, about a quarter of it the search and the rest the two
attribute reads that feed it. That method is now in `doc/todo/31` beside the item.

## The gates

The whole of `doc/todo/02` §2 ran after the last edit: 1704 tests pass, the corpus gate's
incomplete list is unchanged, both text gates are unchanged, quorra is unchanged, and the
conformance checker passes with the new quotations verified against `doc/md/`. The oracle's
verdicts are this branch's base rather than any earlier round's — nothing in this change is on a
drawing path, and the base moved with `main`.

All four new tests were checked by deleting the code they guard: without the scope filter the
column search returns a row's header it should have stepped over; without the "data cell after a
header cell" condition the search runs to the table's edge; with the recursion's loop taken out a
stated array loses the header its own header names; and with the subtree descent removed the header
whose words are in a `P` goes back to being nameless.
