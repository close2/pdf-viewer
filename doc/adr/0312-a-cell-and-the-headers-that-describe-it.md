# ADR 0312 — A cell, and the headers that describe it

Status: accepted, 2026-08-13. Session 477. Amends §14.8.4.8.3's and §14.8.5.7's ledger rows.
Extends ADR 0214's bridge and ADR 0300's grid; changes nothing either decided.

## The question

A screen reader announces a table cell's headers before the cell. Without them a data cell is a
number with nothing in front of it — *23* rather than *Monday, Sydney: 23* — and the whole point of
tagging a table is lost at the last step. ADR 0300 gave a `TH` its axis; nothing said which `TH`
describes which cell.

## What the standard states

The answer is stated twice, and the clause chooses between the two rather than combining them.
§14.8.4.8.3:

> If the Headers attribute (see 14.8.5, "Standard structure attributes") is not specified, any cell
> in a table may have multiple headers associated with it. These headers are defined either
> explicitly by the Headers attribute, or implicitly, by the following algorithm

**The explicit route** is Table 384's entry, an array of the `/ID`s of `TH` elements, with an order
that is part of the answer — "[t]he order in which the entries in the Headers array are listed
shall be row IDs followed by column IDs. The row and column IDs shall be ordered from most specific
to most general" — and a recursion nothing else in §14.8 has:

> Therefore, the headers associated with any cell shall be those in its Headers array plus those in
> the Headers array of any TH cells in that array, and so on recursively.

**The implicit route** is the algorithm itself:

> To find headers for any data or header cell, begin from the current cell position and use the
> current value of WritingMode to search towards the first cell in the appropriate
> horizontal/vertical direction. The search terminates when any of these conditions is reached:

> the edge of the table is reached

> a data cell is found after a header cell

> a header cell has the Headers attribute set -the headers that are specified are appended to the
> row/ column list that is being built

> When a header cell is found in the search and the (implicit or explicit) Scope attribute of the
> header cell is either Both or Row/Column , the header cell is appended to the end of the list of
> row/column headers, resulting in a list of headers ordered from most specific to most general.

Three things follow that are worth stating because each decides code.

**The direction is the grid's rather than the page's.** The clause names `WritingMode` and then its
own NOTE takes the question away from it: "[t]his algorithm works for languages with different
intrinsic directionality of the script (such as right-to-left) because the structure always
reflects the logical content order of the table". So the search runs towards column zero and row
zero of the *structure*, which is the same reading ADR 0300 took for §14.8.5.7's assumption.

**The search is backwards in walk order**, which is not obvious and is the reason no second pass is
needed: both directions run towards the *first* cell, and a tree walk fills a table row by row.

**A data cell before any header cell does not stop it.** The condition is "a data cell is found
*after* a header cell", so the search skips leading data cells, collects a run of header cells, and
stops at the first data cell beyond that run.

## The population, counted before anything was built

`crates/pdf-model/examples/cell_header_census.rs`, over the pdf.js corpus, `doc/corpora/` and
`doc/` — 1251 files, 151 of them tagged, 21 883 `TH` and `TD` elements, every one of them placed in
a grid:

| | |
|---|---|
| cells stating Table 384's `/Headers` | 281, of which **2 state an empty array** |
| identifiers those arrays name | 475 |
| identifiers that resolve | **475**, every one a `TH`, in the same `Table`, earlier in the walk |
| of those, naming a cell that states `/Headers` itself | **188** — the recursion is exercised |
| cells that end with at least one header | **17 431** |
| of those, answered by the *search* rather than by an array | **17 152** |
| header associations in all | 28 770 |
| longest answer for one cell | 23 |
| `TH` stating Table 384's `/Short` | **0 of 6197** |

The first line and the sixth are the argument for doing the work at all: implementing only the
array would have answered for 279 cells of the 17 431 that have an answer, and the ledger row would
have said `/Headers` was read. That is trap 5's shape — a clause with two routes to one answer and
one of them silent — and it is why the census was written before the code.

The last line is the argument for *not* doing something: `/Short` is five lines and a field on the
wire, and its EXAMPLE is precisely this feature ("It can become cumbersome for a user to repeatedly
have to listen to the full contents of a TH structure element"). No document states one. It is
written into `doc/todo/31` with the count beside it rather than built for a population of nothing.

**The longest answer is not a defect.** ISO 32000-2 contains a table whose every cell is a `TH`, so
the two termination conditions never fire and the search collects the whole row and the whole
column — 23 headers for one cell. The clause produces that; the document is what is unusual.

## What was built

**`pdf_model::structure::TableStack` now keeps what it places.** ADR 0300 made it a grid a walk
*drives*; it keeps, per table, the interval of rows each column is occupied by and which cell fills
it, so `covering(row, column)` is a binary search and the search steps over a whole spanning cell
rather than over each of its columns. Memory is one entry per column a cell occupies — a `/RowSpan`
costs nothing — bounded across the walk by `MAX_TABLE_GRID`, which the corpus does not come near
and `TableStack::truncated` reports.

**The headers are answered after the walk, not during it.** `TableStack::headers` is a second call,
and the reason is the standard rather than convenience: nothing in Table 384 makes the cell an
identifier names one the walk has already reached, so the identifiers stay unresolved until the
tree is done. That is also what lets the entry's recursion terminate against a set of cells that is
complete.

**The answer is in *tokens* the caller supplies.** `enter` takes the caller's own index for the
element and `headers` answers in those, so `viewer_core` gets indices into the list it is building
and `pdf-model` names no consumer's type. A header cell pruned as belonging to another page is
dropped rather than pointed at, which is a real loss and is written down as one: a table whose
header row is on the previous page loses it.

**A header found twice is answered once.** Two searches build one list and a cell spanning both
axes can be met by both; the standard says nothing about the case, and naming a header twice would
have a reader say it twice. A choice, recorded as one, and the same set is what bounds the
recursion.

## Where it reaches a person, and the two routes that do not

The clause is only worth implementing if the answer is spoken, and on AT-SPI it very nearly is not.

- **`accesskit::Node::set_labelled_by` is the relation this is**, and `doc/todo/31` said the
  adapter exposes it. It does not: `accesskit_atspi_common::Node::relation_set` builds exactly one
  relation, `ControllerFor`, out of `Node::controls`. Worse than inert — `accesskit_consumer`'s
  `label` *falls back* to the labelled-by nodes' text where a node has no label of its own, so an
  empty table cell would have been announced as its own headers.
- **AT-SPI's `Table` and `TableCell`** are where a client would ordinarily ask, and that adapter
  implements neither. ADR 0300 recorded this for the row and column indices; it is the same gap.

So the headers go into the node's **description**, in the clause's order — *headers, most specific
first: Monday, Sydney* — which is the channel ADR 0300 already used for the axis the platform has no
word for. **That is a choice about a platform rather than a reading of the standard**, and Table
384's own `/Short` is what says the choice serves the clause: "[w]hen accessed by means of a screen
reader, for each table cell the applicable header cells are read to the user in order to allow that
user to understand the content of the table cell."

## What the bus found that no test could

The tree was read back off a real AT-SPI bus, `doc/verify.md`'s recipe, and the first run showed
**every cell in `bug2014080.pdf` with no headers at all** while the unit tests passed. That document
puts each cell's words in a `P` inside the cell, so every `TH` in it has an empty
`AccessibilityNode::name` — which is correct, because that field is deliberately the element's own
text and not its subtree's, and a container repeating its children's would be read twice.

A cell *named as a header* is the one place the subtree is what is wanted, because nothing else
descends into it to say the words. `tree::spoken_headers` builds that text, once per header cell
rather than once per cell that names it, stopping at §14.9.3's `/Alt` for the same reason the
publishing walk does. The second run reads *headers, most specific first: Monday, Sydney* on every
data cell of that table, and `pdfjs_wikipedia.pdf` — whose ten cells take the *stated* array through
an `/IDTree` — reads its own.

**The same instrument found the same shape of defect in ADR 0214** — every `Role::Label` node on the
bus with an empty name — and in ADR 0301, where a node with no bounds implemented no `Component` at
all. Three findings, one lesson: the crate reaching it is not the program reaching it, and only a
client walking the bus can tell the two apart.

## What it costs

`viewer-core --example accessibility_cost` under `valgrind --tool=callgrind`, which is the
instrument this round had to change to: five other rounds were building on the same machine and the
same binary read 56 ms and 151 ms for the same work, so a stopwatch could not see a percentage.
Instruction counts are exact and load-independent. The query's own cost separates from the open by
running it 1 and 11 times and dividing the difference by ten.

| | baseline | with §14.8.4.8.3 | |
|---|---|---|---|
| `Query::AccessibilityTree`, ISO 32000-2 | 382 137 042 | 386 617 522 | **+1.17%** |
| `Query::AccessibilityTree`, `Tagged-PDF-Best-Practice-Guide.pdf` | 30 143 751 | 31 497 618 | **+4.5%** |

About a quarter of that is the search; the rest is the two attribute reads per cell that feed it,
`/ID` and `/Headers`. In wall clock the whole query is 191 µs on `bug2014080.pdf` and 1.7 ms on
`pdfjs_wikipedia.pdf`, which are the documents where the answer is not empty.

**And the measurement found something it was not taken for**, which is written into `doc/todo/31`
rather than fixed here: ISO 32000-2's page 400 answers with **no nodes at all**. The walk stops at
`MAX_NODES` = 8192 elements of the whole document's tree and prunes to the page afterwards, so a
screen reader on a thousand-page tagged document hears nothing past the first few pages and nothing
says so.

## What was rejected

- **Falling back to the search where a stated `/Headers` names nothing.** The clause's condition is
  that the attribute "is not specified", and an array that resolves to nothing is specified. Two
  corpus cells state an empty array and get no headers, which is what the document says.
- **Resolving `/Headers` through Table 354's `/IDTree`.** `Tree::element_by_id` does that already
  and answers with a dictionary, which is not something a host can point at; the resolution runs
  over the cells the walk placed instead, keyed by Table 355's `/ID` on the element. It also works
  for a document that states identifiers without the index Table 354 requires beside them. All 475
  corpus identifiers resolve either way.
- **Carrying the header cells' *text* across the boundary.** A header is a node the host already
  has; a copied string would be a second statement of the same thing that could disagree with the
  first.

## A false claim this round retired

`Tree::element_by_id`'s doc comment said `None` was the answer "for a document with no `/IDTree` —
89 of the corpus's 89 tagged ones". Twelve of those 89 state one, and over the wider corpus 22 of
151 tagged documents do — which is where all 475 of Table 384's identifiers are found. Written in
the round that added the reading, false from that day, and found by measuring the thing it was a
claim about.
