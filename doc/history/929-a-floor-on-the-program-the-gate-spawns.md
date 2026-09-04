# 929 — A floor on the program the gate spawns, and a survey that had to be run

Date: 2026-09-04.
ADRs: [0898](../adr/0898-a-cost-floor-on-the-program-the-gate-spawns.md),
[0899](../adr/0899-three-rows-of-a-survey-that-did-not-survive-being-run.md).
Touched: `tools/pdfref/src/cache.rs`, `tools/pdfref/src/extract.rs`, `tools/pdfref/src/lib.rs`,
`tools/pdfref/tests/end_to_end.rs`, `crates/pdf-model/tests/oracle.rs`,
`crates/pdf-model/tests/text_extraction.rs`, `crates/viewer-core/tests/selection_census.rs`,
`doc/questions/Q28`, two ADRs, this file.
**No pixel moves**: no crate that draws changed, and no crate that draws was touched at all — the
whole diff is a test harness and three test binaries.

Round 927 gave `pdf-vfs`'s two walks a counted cost floor and surveyed the other seven
(ADR 0895 §3); `Q27` put the ranking of those seven to the owner and is open. This round took the
rows that need no ranking: the ones that need no library change.

## What was built

The three most expensive lines in `doc/todo/02` §2 — the oracle and the two text instruments — are
very largely measurements of *other programs*: about 1020 seconds of processor time in `pdftoppm`,
`mutool` and `gs` against 46 in our own pipeline, by `pdfref::cache`'s own module comment. Nothing
counted those spawns.

`pdfref`'s two caches now count them, at the two places the program is actually run:

> **A reference program runs at most once per key per run, and the only thing that may make it run
> again is the cache not having kept what it produced.**

`Runs::repeated <= Runs::unstored`, as `Runs::holds`. Sound by construction: between two runs for
one key the first run's outcome must have been unreadable from the cache, and every way of that
happening increments `unstored` — so the ceiling can only be too generous. No band, no check file,
no clock, so a neighbouring round's load cannot move either side by one.

`Statistics` could not have said this. A lookup that never reaches the cache — the cache off, a
document that cannot be hashed, a renderer that cannot be identified — moves neither `hits` nor
`misses` and spawns the program regardless; and a second miss on one key reads exactly like a first
miss on another. That second blindness is the shape of the regression ADR 0886 hid for four
sessions.

## Proving it can fail

`Reference::command_signature` replaces the output path with `<out>` so that where the harness puts
its artefacts is not in the key. Taken out — the cheapest defect this floor is for, and the dual of
the risk the cache's own module comment is written about — the unit floor fails at
`Runs { ran: 2, repeated: 1, unstored: 0 }` and names the key, while `Statistics` on the same run
reads an unremarkable one hit and two misses.

The population is part of the proof, on session 927's own lesson: the test asks for one page three
times **from two different work directories**, because three requests from one directory would pass
with the defect in place.

## What the floors read on the sequence that gated this round

Every one of `doc/todo/02` section 2's thirty-one lines is green, and the three floored ones read
zero repeats: 6705 programs spawned by the oracle over a cold cache, 958 by the text instrument,
and **8** by the census on the entries the line before it had just written, against 947 answers
from disk. Three orders of magnitude between the cheapest run and the dearest, and the inequality
says the same thing about both — which is the whole case for a count. ADR 0898 has the table.

## What the survey got wrong, which is the round's other half

Three of ADR 0895 §3's rows are not what they were costed at, and two of them would have produced a
gate that passed on the first try — the same trap as 927's first proof, one level up.

- **`selection_census`'s readback cache is never asked.** Measured: forty caret queries over the
  annex leave `hits: 0, misses: 0`. `Readbacks::get` is reached only from the search step, and the
  census issues no `Command::Find`.
- **`pdf-model corpus`'s font half is absorbed one level lower.** An interpretation memoises fonts
  for itself, so a one-page walk asks the shared cache at most once per key: 295 first pages give
  706 lookups, **10 hits**, on three documents. The identity a floor would rest on is false by 355
  besides, because a failed load misses and keeps nothing.
- **`pdf-model corpus`'s decode half is exactly countable and not free.** The arithmetic is an
  identity, three of its four counters cost nothing, and the fourth needs every key the cache ever
  held — about a megabyte on ISO 32000-2, in the shipped reader. `Q28` asks the owner whether an
  instrument may spend memory there, and nothing was added meanwhile, deliberately: half an
  instrument is worse than none.

The row that *does* survive is `pdf-transform`'s gate, where one font cache is shared across pages
and the per-page memo is a different memo per page. That is `Q27`'s recommendation 3 and stays
untouched, because a round does not retire another round's gate on its own reading.

## What was left, and why

Q27's recommendations 1 and 2 — the `accessibility_census` interpretation counter and the
`Document::open` counter that would serve seven gates — are the owner's to rank and were not taken.
`render-quorra` needs a counter inside a graphics backend and is unchanged. `pdf-vfs`'s three
remaining costs are **durations** rather than counts, and ADR 0895 §4 already says what a duration
costs to believe on this machine; they stay unfloored, correctly.
