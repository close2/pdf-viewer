# 1953 — Every page in one walk, and the check that did not end

Session 1558. Status: **accepted** and **built**. Context: ADRs 0330, 1496, 1810; `doc/todo/72`
section 2 ("One validation that does not end").

## 1. The loop

`pdf_archive::check` held one target for minutes on Isartor's `6-1-12-t01-fail-a.pdf`
(4 010 934 bytes): ten thousand pages in one `/Kids` array, built to exceed PDF/A-1's array limit.
Three `gdb` samples at 10, 25 and 40 s were each inside `pdf_model::Pages::get` called from a loop
over `0..len()`. `get(i)` walks a `/Kids` array from its first entry and skips only subtrees it
trusts by `/Count`, so a flat array costs *i* node reads per page and *n*²/2 per enumeration. At
HEAD in `release`, one such pass took 27.4 s (`Examination::pages`), the content survey's 30.9 s,
and each of seven action and presentation predicates 28 to 32 s of its own walk: 269 s per target,
six targets per census row, and longer still in the `debug` build the census ran.

**Batch eighty-eight did not cause it**: its one `pdf-archive` change (the Type 3 `.notdef`
report) touches no page loop, and the enumeration is unchanged since `57da5459` (2026-10-05). It is
a quadratic walk over a file shaped to exceed an implementation limit, not a decode — the brief's
hypothesis held.

## 2. The answer

`Pages::every()` returns what `(0..len()).map(get)` returns, from one depth-first walk that reads
each node exactly as `find_leaf` does and builds every leaf once. It is taken only where it is
provably the same answer: every `/Count` `get` would skip by equals the pages found beneath it
(Table 30 makes that the writer's "shall"), the walk stays inside `MAX_NODES_VISITED` (one lookup
visits a subset of these nodes), and every leaf's dictionary copies out. Otherwise it answers by
the lookups, so no page moves to another index on any file. Every page loop in `pdf-archive` and
`pdf-transform`'s archive verb now enumerates through it.

**The bound is the walk's own.** The check is now linear in the page tree, which is bounded by
the file's bytes and by `MAX_NODES_VISITED`, so no budget of steps is added beside it; a tree with
false counts is still answered quadratically, and that is the residue. Over the Isartor file one
target's report fell from 269 s to 1.5 s in `release` (`pages` 54 ms, survey 73 ms; the dearest
predicate, `page-boundary-sizes`, 1.3 s for its `/Parent` chains).
