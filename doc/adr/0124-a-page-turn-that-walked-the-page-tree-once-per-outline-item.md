# ADR 0124 — A page turn that walked the page tree once per outline item

Status: accepted, 2026-08-01.

## The report

> I have opened the 1023 page big ISO spec and switching between pages is really slow. It is
> fast, if I use a smaller pdf, so something takes a lot of time, when switching to the next
> page.

The user is right, the diagnosis is arithmetic, and the fix is 40× on the document this project
keeps in `doc/` and reads the standard out of.

## Where the time went, measured before anything was changed

Release build, ISO 32000-2, page 500:

| | |
|---|---|
| `Document::open` | 15 ms |
| `Pages::new` + `len` | 0.4 ms |
| `Pages::get(500)` | 3.8 ms |
| `Outline::read` | 5.6 ms |
| **`Outline::section_at(500)`** | **344 ms** |
| `interpret(page 500)` | 13 ms |
| `PageLabels::read` | 10 µs |

One line is 96% of a page turn.

## Why

§12.3.3's outline is a tree of items, each naming a *destination*, and a destination names its
page **by reference**. Turning that reference into an index is `Pages::index_of`, whose own doc
comment has said what it costs since it was written:

> Unlike [`Self::get`], this cannot use `/Count` to skip a subtree — the whole point is that the
> target's position is unknown, so a subtree cannot be dismissed without being searched. It
> therefore costs a walk of the tree in the worst case, which is why it is on the path of a
> person following a link and not on the path of opening a document.

That comment is exactly right and describes *one* lookup. `section_at` — which answers "which
section of the document is this page in", for the title bar — asked it **once per item**. ISO
32000-2 has 988 outline items and 1023 pages, so a page turn walked a thousand-page tree a
thousand times: `O(items × pages)`.

Every part of that was in view. `index_of` says it is a search; `section_at` loops over every
item; and the cost is the product. Nobody multiplied them, because until the
hundred-and-thirty-second session a page turn's cost had never been *asked about* — and because
on a 14-page document the product is 14 × 20 and disappears.

## The fix

`Pages::indices()` gathers every page's object and index in **one** walk, and
`Destination::page_index_with` resolves against that map instead of searching.
`section_at` builds it once for the whole outline: `O(pages + items)` where the loop was
`O(pages × items)`.

Measured after, same test, same sitting:

| | before | after |
|---|---|---|
| `section_at(500)` | 344 ms | **4.5 ms** |
| a page turn, through `viewer-core` | ~380 ms | **~9 ms** |
| `Command::Open` on the spec | 471 ms | **33 ms** |

The open path fell with it, because opening a document announces its first page and therefore
asked the same question. 471 ms of that was `CLAUDE.md` principle 2's "nothing eager" being
broken by an algorithm rather than by an eager read.

`named_page::disagreements` had the same shape — a walk per named page — and its doc comment said
so honestly. It uses the map now too: the same fix, applied before anybody met it on a document
with enough named pages to notice.

## The test, which is a ratio rather than a time

`an_outline_resolves_against_the_page_tree_once` times **one** `index_of` and then the whole
outline, and asserts the second is under ten times the first. It says the algorithmic property
directly — 988 destinations must not cost 988 searches — and it says the same thing on a slow
machine, which an absolute millisecond bound would not. Against the old code it fails with the
numbers in the message: `988 destinations resolved in 418.602086ms, against 1.061116ms for one
search: that is a walk per item`.

The document it runs on is ISO 32000-2 itself, which is committed in `doc/`. The largest thing
this project has, used as a test for the first time.

## The second walk, on the path of a mouse

The same measurement showed two more:

| on ISO 32000-2's page 900 | before | after |
|---|---|---|
| `Query::PageGeometry` — asked on **every frame** | 3.06 ms | **832 ns** |
| `Query::LinkAt` — asked on **every pointer move** | 6.05 ms | **52 µs** |

Both were `Pages::get`, which walks from the root: the page being *shown* was looked up afresh
every time anything asked about it. It is now kept in `Open::current`, and the display list
carries the page's extent so the magnification, the geometry and every mapping of a pointer
position answer from it.

**The subtlety is where the cache lives**, and the first version got it wrong: putting the page
inside `Interpreted` tied it to the display list's lifetime, and the display list is thrown away
whenever the page's ink changes — a layer switched, a value typed, §12.5.5's appearance following
the pointer. A press invalidates the appearance *and then asks what it landed on*, so the cache
was empty exactly when it was needed and `a_click_on_a_link_shows_the_page_it_names` failed. The
page has not changed at any of those moments; the picture of it has.

## The lesson

**A cost written down beside one call is not a cost anybody adds up.** `index_of`'s comment
named the walk and named the two paths it was for — a link, and the open action — and a third
caller arrived that was neither, looped, and inherited the comment's blessing without inheriting
its argument. The defence is to ask, of a function documented as expensive, **who calls it in a
loop** — one `grep`, and it also found `named_page`.

The second test is the same shape: twenty queries about the page on the screen must cost less
than **one** walk of the tree. With the page kept they cost an eighth of one; with it looked up
each time, twenty. Reverting the single line that reads the cache fails it.

And: **the gates could not see this.** The corpus opens 974 documents and interprets page one;
the oracle renders 1794 pages it is given by index; neither turns a page in a viewer, and the
largest document in the tree is not in either. **A performance defect on a path no gate walks is
found by a person using the program**, which is what happened, and the arithmetic was visible from
the first `println!`.
