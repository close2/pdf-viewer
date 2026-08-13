# A cold document-wide search, and a fifth of it is the page tree

Status: **raised by the project owner on 2026-08-09**, on reading session 414's report:
*"Is the search implemented single threaded? 6.19s doesn't sound that fast. Can we easily improve
this? … Any improvement must be reasonable. Improving the search speed is a goal, but not a
requirement if the cost is too high (for instance in code quality or possibly also memory usage)."*
**Half answered in the four-hundred-and-twentieth session** (ADR 0256): the *repeated* search is
750× cheaper and the window's is **0.021 s** against about five seconds, and the *cold* search was
unchanged. **The cold one moved in the four-hundred-and-eighty-second** (ADR 0317), by memoising
§7.4's filter chain — a third of the instructions off a hundred-page sweep. What is left of it is
§7.7.3.2's page tree, below.
Priority: 47 — performance, measured, and explicitly **not** a requirement: the owner has priced
the trade in advance and code quality and memory both outrank the seconds
Corpus: every document; the cost scales with the *document*, not with the needle
Code: `crates/viewer-core/src/viewer.rs` (`find_step`, `readback`),
`crates/viewer-core/src/readback.rs`, `crates/pdf-model/src/page.rs` (`Pages::get`),
`crates/pdf-model/src/content.rs`. `crates/viewer-core/examples/find_cost` is the instrument.

## Where the time goes, measured

`find_cost` drives the pump with no host at all, so the number is the search's. ISO 32000-2, 1023
pages, `--profile gates`, medians of three:

| one cold sweep, split | | |
|---|---|---|
| §7.7.3.2's page tree, `Pages::get` once per page | **1.12 s** | **19.6%** |
| the content stream, `interpret_with` | **4.59 s** | **80.4%** |
| building the index, `Pages::new` × 1023 | 2.8 ms | 0.05% |

And end to end, medians of seven, against the same binary with the cache's budget set to 0:

| | before | after |
|---|---|---|
| first (cold) sweep | 5.51 s | 5.61 s — **unchanged**, inside spreads of 0.16 and 0.32 s |
| repeated sweep | 5.45 s | **7.27 ms** |
| in the window (`Xvfb`, lavapipe), first / repeated | ≈4.9 s / ≈4.9 s | **4.79 s / 0.021 s** |

## What was done, and what it settles

**Candidate 3, the readback cache, is built** — `crates/viewer-core/src/readback.rs`, 4 MiB per open
document, least-recently-used, observable through `Viewer::readback_cache` and
`pdf-viewer --trace=search`. ADR 0256 has the argument, the invalidation rule and the cost; the
short of it is that ADR 0250 declined it against a bound nobody had stated, and the owner stated
one.

**Candidate 1, "interpret less", is answered rather than tried**, and the measurement is why: a
text-only narrowing of the walk could remove at most part of the 80.4%, and the cache removes
*all* of both halves for every page after the first. What it cannot do is help a *first* search
— and a second extraction path would be the wrong way to buy that, for the reason this file has
always given: it can diverge from what the page draws, and then a search finds words the reader
cannot see. **What did help a first search was inside the 80.4% rather than beside it**, and it is
the section below: the same font program inflated once a page.

**Candidate 2, parallelism, is measured and declined for now** — the four-hundred-and-twenty-fourth
session, ADR 0260. The blocker this file named was not one: **N documents in N threads needs nothing
from `pdf-syntax`**, and it reads 1023 pages in **1.18 s against 6.11**. `Document` is `Sync` now
(0.021% of a page interpretation's instructions, nothing measurable in wall clock) and one
`&Document` on 24 threads does it in **1.61 s** — but the peak resident memory is **625 MB shared or
966 MB per-thread against 225**, and the owner's stated bar is that 1 GB is definitely too much. So
what is open is `doc/todo/49` item 3, the API that hands a pool in, and it is a memory argument
rather than a lock. `pdf-model/examples/parallel_sweep` is the instrument.

**Candidate 4, skipping pages by scanning bytes, is still unsound** for the reason it always was.

## And a second one, found by counting: nothing memoised a decoded stream — now something does

**`Document::decoded_stream_data` ran 12 717 times over one sweep of ISO 32000-2 and 11 975 times
over the *second* sweep of the same document** (ADR 0260's counter build). That is §7.4's filter
chain re-run — a flate inflate per content stream, per font program, per image — on a document
nothing has changed.

**Taken in the four-hundred-and-eighty-second session, and it is the first thing on this file that
makes a *cold* search faster.** The two things the round owed were the two this file asked for. What
it is worth, measured before anything was built: **24.6% of a cold sweep is inside
`decoded_stream_data` and 23.4% of it is decoding something already decoded** — 830 MB of
re-inflation against 46 MB of first decodes, three font programs accounting for 3.2 s of the 3.9.
The budget is 4 MiB per open document, derived from the owner's band less what the readback already
spends and from a least-recently-used replay that says 4 MiB is 0.3 points short of unbounded.
ADR 0317 has the census, the derivation, the liveness invariant that makes an address a legitimate
key, and the callgrind A/B: **4 933 481 135 → 3 133 405 696 instructions over a hundred-page sweep,
−36.5%**, with the readback byte-identical.

**Wall clock was not the instrument and the reason is worth keeping.** The machine carried a load
average of 20 to 30 that session, and seven interleaved samples an arm gave medians of 9.75 s
against 6.73 s with ranges of 9 s and 8 s — the right direction and no evidence at all. A
measurement that has to survive a busy machine counts instructions.

## What is left: the page tree, and it is not a cache question

**A fifth of a cold search is walking §7.7.3.2's tree from the root, once per page** — and it is
more than a fifth now, because the section above took a third of the instructions off the other
half and this one lost nothing. Re-measure the split before quoting the share again. `Pages::get`
descends for every index — 1.10 ms on the thousandth page of ISO 32000-2 — so a sequential sweep is
the access pattern a tree walk is worst at, and it is the same walk `Open::page` makes on every page
turn. `Pages::new` is *not* the cost (2.7 µs), so hoisting the index out of `Open::page` buys
nothing measurable and is not worth the lines.

What would buy something is an index the walk builds as it goes: the first descent to page *n*
passes every node on the way, and remembering the leaves it saw would make the sweep O(tree) rather
than O(pages × depth). Two things a round taking this owes:

- **`CLAUDE.md`'s startup rule is on the other side of it.** "No full page-tree walk" on the launch
  path, and "a 500-page document must open no slower than a 5-page one" — which is a rule this tree
  measured itself against in session 289 and holds today (1023 pages and 5 pages cost the launch the
  same 5 ms of join). A *lazy* index filled by walks somebody asked for does not break that; an
  eager one does, and the difference has to be in the design rather than in the comment.
- **Measure `find_leaf` before believing the 1.12 s is all descent.** The split above times
  `Pages::get` whole, and that includes §7.7.3.4's inheritance being applied from the root on every
  call. Which of the two dominates decides whether the answer is an index of leaves or a cache of
  resolved `Page`s, and those have very different memory costs — a `Page` holds dictionaries, where
  a readback is 2.6 KB of text.

## What a round taking this still owes

- **Measure `find_step` alone**, not a host's loop. `find_cost` is the instrument now and prints the
  split itself with a fourth argument.
- **The gate is unchanged**: `tests/text_extraction.rs` at 99.2% and the 14 specification PDFs at
  100% of `pdftotext`'s words, both exactly. A search that returns different results is a defect.
- **Do not undo the pump.** Rules 3 and 4 are not negotiable.
- **State what it costs in memory and in lines.**

## What is explicitly not owed

A match count ("3 of 17"), which needs the whole document read before the first answer. It is now
*cheap on a second search and unchanged on a first*, which is a different shape from the one ADR
0250 declined — but no host has asked, and nothing in the vocabulary prevents one doing it.
