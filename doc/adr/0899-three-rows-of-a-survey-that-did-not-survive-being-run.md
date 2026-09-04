# 0899 — Three rows of a survey that did not survive being run

Session 929. Status: **accepted**. The second of this round's two records:
[ADR 0898](0898-a-cost-floor-on-the-program-the-gate-spawns.md) is the instrument and the floors;
this is what happened to the rest of ADR 0895 §3's table when a round tried to build from it.

## The rule this is an instance of

`doc/habits.md`'s *Measuring* section says a price decays, and ADR 0895 §3 said so about its own
table: "a round taking one of these re-derives it rather than believing the table". This round
took four of the seven rows and **three of them are not what the survey costed them at** — each
for a reason the survey could not have seen without running something, and each an instance of the
trap session 927 named on its own first proof: *a floor whose population cannot reach the defect is
trap 25 with a counter on it.*

Nothing here contradicts ADR 0895's finding, which was that the counters exist more often than the
assertions. What decays is the second half of each row: **which population would move the counter.**

## 1. `selection_census` — the counter the census never asks

The survey's row: *one page interpretation per document, however many carets are walked —
`Viewer::readback_cache` already reports hits/misses/evicted and this test reads neither*, needing
**nothing**, the accessor being public already.

Measured. The census opens a document, selects everything, and walks carets over page one; the
readback cache after forty caret queries on `doc/PDF20_AN001-BPC.pdf` reads:

```
ReadbackCache { pages: 1, bytes: 173, budget: 4194304, hits: 0, misses: 0, evicted: 0 }
```

**Zero and zero.** `Readbacks::get` is reached from exactly one place — `Viewer::readback`, the
search step — and the census issues no `Command::Find`. The entry is there because settling a page
*puts* one; nothing ever asks for it. An inequality over `misses` on this gate is a clean answer to
a question the population does not put, and it would have passed for ever.

The row is not wrong about the cache. It is wrong about the gate: `readback_cache` is the right
instrument for `viewer-core/examples/find_cost` and for `--trace=search`, which is where it is
already read.

## 2. `pdf-model` `corpus`, the font half — absorbed one level lower

The survey's row: *one font load per distinct font dictionary*, test-only via `interpret_with_fonts`
with a locally owned cache.

That property is real, and the gate cannot see it, because **an interpretation already memoises
fonts for itself**. `content::font`'s loader is two levels deep: a per-interpretation map keyed by
`FontKey`, and only under that the `FontCache` that outlives the page. So within one page the
shared cache is asked at most once per key however many `Tf` operators name it — and the corpus
gate interprets exactly one page per document.

Measured over the first 295 documents of `doc/pdf.js` that open with a page one, each with its own
`FontCache`:

```
295 documents: 706 font-cache lookups, 10 hits, 696 misses, 341 held; 3 documents saw a hit at all
```

Ten hits in seven hundred lookups, on three documents — those are pages where two resource names
resolve to one font object, which is the only way a single page can ask twice. Bypassing the shared
cache entirely would move none of the other 696.

The same figures kill the identity a floor would have been built on. `misses` is 696 against 341
held, because **a failed load misses and keeps nothing**: `FontCache` holds successes only, by a
decision `cached_font` argues (a kept failure would change the second page's *reports* and not only
its cost). So `misses = fonts + replaced + evicted + declined` is false by 355 on this population
before any defect is planted.

**Where the row does hold is `pdf-transform`'s gate**, which is a different line and Q27's
recommendation 3: there one `FontCache` is shared by every page of a document, the per-page memo is
a different memo per page, and "one font cache, shared by every page" is exactly what a count of its
misses can state. That row is untouched by this finding.

## 3. `pdf-model` `corpus`, the decode half — countable, but not for free

The survey's row: *one decode per distinct filter chain (`Document::decoded_streams()` — hits,
misses, evicted)*, test-only.

The property is the sharpest of the seven and the arithmetic is exact. `DecodedStreams` keeps one
entry per `(address, length)` of the encoded bytes; every miss is followed by one `keep`, which
either stores, replaces, or declines an entry larger than the whole budget; entries leave by
eviction, by replacement, and by `clear`. So

> misses = held + replaced + evicted + cleared + declined

and `repeated ≤ forgotten` follows exactly as it does in `pdf-vfs`. **What the report cannot supply
is the number of distinct keys**, and there is no way to it that does not remember every key the
cache has ever kept — a set that grows with the document rather than with the budget, in
`pdf-syntax`, which is the shipped reader. On ISO 32000-2's 1023 pages that is on the order of a
megabyte of instrument in the program a person runs, held for the life of an open document, against
a `DECODED_BUDGET` whose whole design is that it be legible and bounded.

Three of the counters that identity needs — `replaced`, `cleared`, `declined` — are free, and they
are worth having. The fourth is not free, and buying it is a `CLAUDE.md` principle 2 decision rather
than a round's: `doc/questions/Q28` asks it.

## 4. What this round did build, and what it leaves

Built: the counter on the program a gate spawns, and the floor on the three §2 lines that spawn
one (ADR 0898).

Left, with the reason:

| walk | left because |
|---|---|
| `viewer-core` `accessibility_census` | Q27's recommendation 1, unranked by the owner, and needs a counter at `viewer_core::open`'s interpretation funnel |
| the six `pdf-transform` walks | Q27's recommendation 2 — the `Document::open` counter — unranked |
| `pdf-transform` `gate` | Q27's recommendation 3, which proposes *replacing* the pages-a-second floor; a round does not retire another round's gate on its own reading |
| `render-quorra` `corpus` | a counter inside a graphics backend, the least tractable of the seven and unchanged by anything here |
| `pdf-model` `corpus` | §2 and §3 above: one half cannot discriminate, the other costs memory in the shipped reader (Q28) |
| `pdf-vfs`'s three durations | ADR 0895 §4 — a duration is not a count, and ADR 0884 is what one costs to believe on this machine. Unchanged and correctly left |

## Consequences

- ADR 0895 §3's table keeps its two general findings, which measurement did not touch: the
  instrumentation exists more often than the assertions, and `Document::open` is the highest-leverage
  counter this tree does not have.
- Its per-row costings are now known to be a *reading* rather than a measurement for at least three
  of the seven, and Q27's ranking should be read with that. The correction is here rather than in
  that file, because an ADR is not edited to follow the tree that moved under it (ADR 0232 §2).
- The general lesson is trap 13's, one level up from code: **a survey of what an instrument would
  catch is itself a claim that has to be run.** Two of the three rows above would have produced a
  gate that passed on the first try, for the same reason session 927's first proof did.
