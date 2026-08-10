# ADR 0256 — A readback kept under a bound, and the progress count that outlasted it

Status: accepted, 2026-08-10 (session 420).

## Context

ADR 0250 built the document-wide search and declined to keep anything:

> **Nothing is cached.** A `next` that crosses three pages interprets three pages, and a second
> search over the same ground interprets them again. The alternative is a readback cache with a
> byte budget and an invalidation rule for every view-state change that alters what a page draws
> — a layer switched, a value typed, an appearance following the pointer — and at 5.7 ms a page
> nobody has measured a need for it.

The project owner read the resulting 6.19 s and asked whether the search was single-threaded, and
then settled the memory question that the paragraph above was really about:

> I am also not against caching text for a search (and therefore using memory), it should just not
> use too much. (1 GB is definitely too much, below 10 MB is definitely ok.)

`doc/todo/49` records what that decides. **The byte budget ADR 0250 was missing was a number, and
the number had already been measured in the same ADR**: the readback of ISO 32000-2's 1023 pages is
2.66 MB, four times inside "below 10 MB is definitely ok", for the largest document this project
owns. A decision taken against an unstated bound is not a decision; it is a preference. So the
question stopped being *whether* and became *what shape*.

## Where the time actually goes, measured before anything was built

`doc/todo/47`'s standing warning is that the first measurement of this feature measured something
else — 19.25 s of which 13 s was `viewer-ui` presenting a whole window per step. So the instrument
is a new one that drives the pump with no host at all: `viewer-core/examples/find_cost`, which
opens a document, pumps `Find::Start` and `Find::Continue` to the end, renders nothing, and prints
the sweep. A needle the document does not contain is the worst case and the only one whose cost is
a property of the *document* rather than of where the word happens to sit.

ISO 32000-2, 1023 pages, `--profile gates`, medians of three:

| one cold sweep, split by hand | | |
|---|---|---|
| §7.7.3.2's page tree, `Pages::get` once per page | **1.12 s** | **19.6%** |
| the content stream, `interpret_with` | **4.59 s** | **80.4%** |
| building the index itself, `Pages::new` × 1023 | 2.8 ms | 0.05% |
| readback produced | 2 658 697 bytes | |

Two things follow, and only the first is this round's.

**The 80% is what a cache reaches and the 19.6% is not.** Both are per *step*, so keeping the step's
answer removes both — but only for a page read a second time. Nothing here makes a *first* search
faster.

**A fifth of a cold search is walking the page tree from the root, once per page.** `Pages::get`
descends §7.7.3.2's tree for every index, so a sequential sweep is the access pattern a tree walk
is worst at: 1.10 ms per page on the thousandth. `Pages::new` is not the cost — 2.7 µs — so hoisting
it would buy nothing. What would buy something is an index the walk builds as it goes, and that is a
change to `pdf_model::Pages` with `CLAUDE.md`'s startup rule on the other side of it ("a 500-page
document must open no slower than a 5-page one"). **Declined this round and recorded in
`doc/todo/47` as the next candidate, with the number attached.**

## Decision

### The readback is kept, in `viewer-core`, beside `interpret` and never inside it

`crates/viewer-core/src/readback.rs`: page index → readback, a byte ceiling, least-recently-used
eviction, and a report a person can read. It hangs off `Open`, which is the viewer's per-document
state, so it is dropped with the document and no other document can see it.

**Not in `pdf-model`.** `pdf_model::content::interpret_with` is a pure function of a document, a
page and a view state, and the oracle's 1794-page comparison means something only because it is —
`doc/todo/49` puts that in the "keep, not negotiable" list. A cache *beside* it breaks none of that,
because purity is a property of the answer rather than of how fast the answer is reached: the same
three inputs still give the same `Interpretation`, and nothing in `pdf-model` gained a `&mut`, an
interior mutability or a lifetime.

**Only `Interpretation::text`.** Not the display list, not the text layer, not decoded images. That
is the whole reason the number is small enough to argue about: one page of ISO 32000-2 reads back
as 2.6 KB and its display list is larger than the entire readback of the document.

### The bound is 4 MiB per open document, and eviction never runs on anything this project has

One constant, one place, `readback::BUDGET`. The largest document this project owns reads back as
2.66 MB and the corpus's largest is `freeculture.pdf` at 352 pages, so the ceiling holds the whole
of the worst case with 1.5 MB spare and `evicted` is 0 on every measurement in this document. Four
documents open at once is 16 MB, the same order as the owner's band.

This is the fourth bound of this shape in the tree — `MASK_BUDGET` at 32 MB, the confined worker's
4 GiB address-space ceiling, `MAX_SAMPLES` halved on measurement in session 396 — and the argument
is the one those three share: **a reader should expect a large document to cost memory, and what
makes that acceptable is that the cost is bounded and legible, not that it is small.**

Eviction is a scan for the oldest entry rather than a second index ordered by use. Two maps kept in
step would be more lines and one more thing to get wrong, and the loop runs only for a document
whose readback exceeds the budget — which is none of them — where it is a walk of a few thousand
`u64`s against the 5 ms of interpretation that filled the entry being dropped.

### It is observable, and by a method rather than by a message

`Viewer::readback_cache(DocumentId) -> Option<ReadbackCache>`: pages held, bytes, budget, hits,
misses, evicted. `pdf-viewer --trace=search` prints it when a search ends, and `find_cost` prints it
after a sweep.

**Deliberately not a `Query`.** That enum is matched exhaustively by six consumers and crosses
`viewer-confined`'s wire; a variant there would cost every host a compile error and the confined
protocol a message, for a number no interface displays. A `Query` is a question a host asks in order
to *draw* something, and this is an instrument. It cost nobody a line.

### Every view-state change empties the whole cache, and that is the conservative half of the trade

`Open::stale` is the one place that says "the ink is stale", and it now means two things rather than
one: the display list of the page showing, and the readback of every page. It is called from the six
sites that previously wrote `interpreted = None` — §8.11's layer switch, §12.7.5's field value
through `replay`, §12.5.5's appearance under the pointer, §12.6.4's action, §12.5.3's `NoZoom`
magnification and §6.3.2.2's delegated widgets.

**Every page, not the page that changed.** The precise version — forgetting only the page an
annotation sits on — is available and was declined: it needs an invariant about which page that is,
maintained at two call sites, and it is wrong the moment a page turns with a pointer appearance
still recorded. `doc/todo/47`'s gate is that *a search returning different results after the change
is a defect and not a speed-up*, and that makes the trade one-way. What it costs is that a layer
switch or a rollover crossing makes the next search cold again, and that is written down here rather
than discovered later.

`settle` puts the page it has just interpreted *into* the cache, which is `doc/todo/49`'s fourth
item: the page a person is looking at was read to draw it, and a find bar's search starts there. One
2.6 KB copy per page turn against 5.4 ms of the thing it saves.

### No `--cache-text` flag, and the measurement is the reason

`doc/todo/49` named the flag and also named what a flag may not be: "a knob whose default is wrong is
a decision deferred onto the user". 4 MiB holds every document this project has, entirely, with
nothing evicted; there is no document in reach whose owner would want to type a number. **The flag is
declined until a measurement produces one** — a document whose readback exceeds the budget, where LRU
under a forward sweep is exactly the pathological case and `evicted` would say so out loud. That
condition is written into `doc/todo/49` so the next round can see it has been met.

## What was measured after, and one number that had not been the subject

### `find_step` alone — `viewer-core/examples/find_cost`, ISO 32000-2, medians of seven runs

The "before" column is the same binary with `BUDGET` set to 0, so the two differ in the cache and in
nothing else, on the same machine within the same ten minutes.

| | before | after |
|---|---|---|
| **first (cold) sweep**, 1024 steps | 5.51 s (5.50 – 5.66) | 5.61 s (5.42 – 5.74) |
| **repeated sweep**, same 1024 steps | 5.45 s (5.37 – 5.50) | **7.27 ms** (6.13 – 8.16) |
| peak RSS, one sweep | 209.6 MB | 211.9 MB |
| peak RSS, two sweeps | 234.0 MB | 211.9 MB |
| readback held at the end | — | 2 658 697 of 4 194 304 bytes, 0 evicted |

**The cold sweep did not move.** 5.51 against 5.61 is inside spreads of 0.16 s and 0.32 s, and an
honest reading of it is "unchanged", not "0.1 s slower" and certainly not "faster". That is the
expected result and it is reported as one: a first search interprets every page either way.

**The repeated sweep is 750× cheaper**, which is a difference three orders of magnitude outside any
spread and needs no statistics.

**Memory went up by 2.3 MB and the peak went down by 22.** The first is the cache: 2.66 MB of
readback held where the allocator would otherwise have reused the space. The second is what two
sweeps used to cost — a thousand more interpretations, each allocating and freeing a page's working
set — and it is worth reporting because "a cache costs memory" turned out to be true only of the
single-sweep case.

### Then the window, and the thing that had not been the subject

`pdf-viewer` under `Xvfb` at 1100×1200 with `lavapipe`, ISO 32000-2, `/` then a word the standard
does not contain, timed from the key press to the line the search prints. Medians of three:

| | with the cache, `SEARCH_REDRAW` = 16 steps | with the cache, `SEARCH_PROGRESS` = 100 ms |
|---|---|---|
| first search | 4.94 s | **4.79 s** (4.756 – 4.789) |
| repeated search | 0.51 s | **0.021 s** (0.018 – 0.026) |

A repeated search that cost 7.27 ms inside `viewer-core` cost **0.51 s** in the window, and all of
the difference was the progress count. `SEARCH_REDRAW` repainted the whole window once every 16
steps; at 13 ms a present under lavapipe that is 64 presents, or 830 ms, of moving a digit — and it
was the right constant when ADR 0250 chose it, because a step then cost 5.7 ms and 64 presents were
a sixth of the search.

**A step count is a proxy for time that was calibrated against one step cost.** When the step cost
fell by three orders of magnitude the proxy stopped tracking the thing it stood for, so the constant
is now the thing itself: repaint the progress count at most ten times a second. It costs the same on
a cold sweep — 48 presents in 4.8 s where 64 was the old number — and nothing on a warm one. The
clock is the *host's*, which is where `doc/ui-boundary.md` puts one: rule 3 leaves `viewer-core`
without one precisely so that a host may spend its own.

**This is ADR 0250's own lesson arriving a second time** — "the expensive part of a feature is often
not the part with the clause number on it, and only running the program says which" — and the second
time it was the fix from the first round that had become the expensive part.

## What it cost

**In memory**: 4 MiB per open document, of which the largest document this project owns uses 2.66 MB.
Peak resident memory on a single full-document sweep rose 209.6 → 211.9 MB and on two sweeps fell
234.0 → 211.9 MB.

**In lines**: **188 added and 33 removed** across `viewer-core/src` and `viewer-ui/src`. Of those,
`readback.rs` is 108 lines of shipping code and 69 of unit test; `Open` gained one field and the
`stale` method that six call sites now share; `Viewer` gained one method and one private helper;
`viewer-ui` gained a trace topic, a field and a clock. `search.rs` gained **nothing but a corrected
comment** — `Searching::step` is handed a `&str` and has never known where it came from, which is
what a boundary is for. Two integration tests (128 lines) and one example (147, not shipped) are the
instruments.

**In risk**: one, and it is named above. A cache keyed by page is wrong if the readback of a page can
change without `Open::stale` running. The six sites are the whole set today; a seventh added without
calling `stale` would be a silent defect, which is why there is exactly one method and not six
assignments.

## Proof the readback did not move

The gate `doc/todo/47` names, run after the change:

- `tests/text_extraction.rs` over the pdf.js corpus: **99.2% (23987/24187 words), 25 below 90%** —
  identical to the run before it, numerator and denominator both.
- The fourteen specification PDFs: **100.0% (841/841 words)**, every document at 100.0%.
- Two new integration tests in `viewer-core/tests/headless.rs` compare the answers directly: a
  second search over the same ground returns the same page and the same range, in the same number of
  steps, **with `misses` unchanged** — so the second answer was computed from what the cache held
  rather than from a fresh interpretation that happened to agree.
- The oracle, the corpus, quorra, dates, XMP and JPEG 2000 gates are line for line what they were.

## A ledger row that had been wrong for three hundred and eighty-four sessions

`doc/todo/02` §1 asks a round to take from the spec track as well, and §9.4.4 and §9.10 are what a
readback *is*. Reading their rows against the code found §9.4.4 `partial` on this sentence:

> The vertical branch is not: ty is always 0, because nothing here reads a vertical writing mode
> (see §9.2.4).

§9.2.4's own row, two clauses above it, says "[b]oth writing modes, from the thirty-sixth session".
`Interpreter::advance_step` takes a `vertical` flag from `Font::is_vertical`,
`pdf_font::LoadedFont::vertical_metrics` supplies §9.7.4.3's `w1`, and the clause's rule — "a
combined displacement shall be computed, denoted by t x in horizontal writing mode or t y in
vertical writing mode (the variable corresponding to the other writing mode shall be set to 0)" — is
the `if`. The row was `partial` on the strength of one sentence and nothing else.

The row is `implemented` and has a test that reads the *text layer* rather than the metrics: two
consecutive glyphs of one string in `vertical.pdf` share a column and descend. It was checked
against the mutation — swapping the branch to translate in `x` fails it — because a test that
asserts what the code does is not a test.

`tools/spec-errata` finds no stale quotation under either §9.4.4 or §9.10. The ledger moves 400 → 401
`implemented`, 252 → 251 `partial`, 875 rows unchanged.

## What is left open

- **The page tree, 19.6% of a cold search**, with the number above and the startup rule on the other
  side of it. `doc/todo/47`.
- **`Document`'s `!Sync`**, which is what blocks cross-page parallelism and is its own measurement
  round. `doc/todo/49` item 1, untouched here by instruction.
- **`--cache-text`**, declined with the condition that would revive it.
- **A match count**, still not built and still nobody's ask — though it is now cheap on the second
  search and expensive on the first, which is a different shape from the one ADR 0250 declined.
