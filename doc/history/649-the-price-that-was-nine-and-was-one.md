# 649 — The price that was nine, and was one

Seventh merge round, four branches, one conflict, and the first batch in this run where **a gate's
agree count went up**. It is also the batch that lost about half an hour to a server-side API
overload and lost nothing else, which is worth recording once because the arrangement was never
designed for it and held anyway.

## What was merged

`round-645`, `round-646`, `round-647`, `round-648`, branched from `fe9653eb`. The one conflict was
two rounds adding to `doc/todo/01` at the same place: 648's is a `###` continuing the count rule
directly above it and 645's a new `##` section, so 648's goes first and both survive whole.

## The sequence, whole, on a quiet machine

| | before | after |
|---|---|---|
| **oracle** | 907 agrees, 66 contradicted | **908 agrees, 65 contradicted** |
| **`render-quorra`** | 933 agree, 23 differ | **933 agree, 22 differ** |
| `nextest --workspace` | 2364 | **2381 passed, 17 skipped** |
| `fixed_documents` | 33 | **35 checked, 0 absent** |
| corpus | 974 documents, 68 incomplete | unchanged |
| fmt, clippy `-D warnings`, fuzz `--bins`, doctests, conformance (163), text, both censuses, dates, XMP, JPEG 2000, `deny` | | all clean |

Ledger unchanged at 875 rows, no `silent` row. `--bin overstated`, the batch's new sweep, prints
**8** contradictions on the merged tree against 9 on 645's branch — the difference being a row 648
corrected from the other side.

## 646: a price that was wrong by a factor of nine, in the cheap direction

Session 643 measured `colors.pdf` against its closed form, found `render-cpu` quantising an edge's
coverage to a quarter — and **to nothing below an eighth of a pixel**, the wrong side of §10.7.4's
*"The area covered by painted pixels shall always be at least as large as the area of the original
shape"* — and priced the fix at **nine `scan::fill` calls where there is one**, on nearly every page.
That price is why it left the item open.

**The price was one call.** `tiny-skia` already contained the nine pieces: an axis-aligned rectangle
has a *rectangle* scan converter beside the supersampled path one, and the geometry §10.7.4 states
is a product of two half-open intervals, so `coverage(i,j) = overlap_x(i) · overlap_y(j)` exactly.
Measured on an idle machine, twenty repeats, both arms built in one sitting, counts confirmed
load-immune against a pass taken at load 103:

| | |
|---|---|
| ISO 32000-2 p101, text | **−0.43%** |
| ISO 32000-2 p6, page-wide clip | **−9.64%** |
| `colors.pdf` p1 | **−7.99%** |

**Exact and faster.** One de-optimisation was found and paid inside the round — a first version
asking the closed form per pixel cost **+33%**, and the run-filled interior is the whole difference
between that and −7.99%.

**And the clip region was not in the brief.** §10.7.4 says the region "consists of the set of pixels
that would be included by a fill operation", so measuring only the fill broke the same paragraph's
`S ∩ C = S` by **26 levels of 255** — caught inside the round by `clip_intersection.rs`, a gate
written years earlier for a different sentence of the same clause.

**643's prediction landed to the fourth decimal.** It had said the exact form would still fail the
gate's bound on `colors.pdf` at ssim 0.98772 / 0.98001, computed with no code and no renderer; the
built form measures **0.9879 / 0.9802**. The pages stay contradicted, because the consensus pair are
the two renderers furthest from the geometry — which is what principle 5 says a consensus is worth.

## 647: the crawl is finished

**65 944 of 65 944 ranked**, nine chunk rounds, and `doc/todo/03`'s header says so. The
whole-population figures exist for the first time: **65 703 open, 720 report anything, 64 939 silent
(98.9%)**. Fifty-nine of the 720 are budgets — 48 `MAX_TILES`, 5 `MAX_FORM_DEPTH`, 4
`MAX_OPERATIONS`, one each state depth and operands — and **no page in the crawl exhausts the clip or
soft-mask tables**.

Its defect is about a refusal's *shape* rather than a budget's value: `MAX_TILES` was checked **in
front of** the cell's interpretation, so a fill that could afford 4096 sites was given none. §8.7.3.1
puts the requirement on the processor — "shall paint the cell … as many times as necessary to fill an
area" — so a budget decides how many, not whether. The sharp part is that **§8.7.3.1's own row already
recorded §7.8.2's prefix rule for the cell's content stream** while the lattice threw its prefix away.
Two things make a tiling; the rule had reached one.

## 645 and 648: the two ways a row goes wrong, and they find different rows

**645 built the eighteenth sweep** — `--bin overstated`, the first that opens **no source file at
all**: a parent asserting an entry or a table *is read*, against a descendant denying it, both sides
sentences this project wrote about its own code. Its first run found nine contradictions over 170
asserting rows and two defects, including **§9.9.1 saying Table 125's three lengths were "read by
nobody" while §9.9's own row had recorded a reader for twenty sessions** — a parent that had outgrown
its child. The discriminator it rejected is priced in ADR 0475, and the reason is the good one: a
tree-facing matcher **would not have found §12.11**, because source comments cite table numbers
freely and it would have reported the claim corroborated.

**648 found what no ordering could.** §12.7.6.2 and §12.6.4.3 carried the same dead test citation
session 626 had fixed for three sibling rows and left in two — twenty-two rounds on. The second could
not have surfaced by blame rank, because its note has been rewritten since; it came out of
**enumerating `refused`'s ten arms against the ledger**. That is the fourth round running where
enumerating a feature's call sites paid, and the clearest statement yet that ordering and enumeration
find structurally different rows.

**648 also corrected a neighbour's number, and the rule is general**: its own census's first draft
visited only xref-listed objects and reported zero `/S /GoToR` over a corpus holding one, because an
action written *directly* inside its annotation has no object number. The same bound is behind
626's "exactly one document states a `/S /Launch` action", which is **two**. *A count over the corpus
is a claim about a walk as much as about the world* — probe a zero before believing it, cheapest
probe first.

## What the interruption cost, and what it did not

All four rounds died within minutes of each other on a 529 overload, and every resume failed for the
next half hour. **Nothing was lost**: no round had committed, each worktree held its full working
state, and `main` was never touched. Two rules came out of it and both are cheap:

- **A resumed round re-reads its own diff rather than trusting memory of it**, and finishes or
  reverts any part-edited ledger row — a half-corrected note is worse than an uncorrected one,
  because the next round reads it as settled. `doc/todo/02` §6's rule, applied to a crash.
- **645's own process failure**: the trap-13 plant restores with `git checkout --`, which destroyed
  its two ledger corrections when run *after* them. **Plant before you correct, or plant into a
  copy.**

And one measurement the batch produced by accident, now worth quoting whenever a round wonders
whether to believe a gate: the oracle took **70.9 s idle against 211.3 s loaded**, same tree, same
verdicts.

## Owed

- **`doc/todo/11` item 7's remainder**: everything that is not a *single* axis-aligned rectangle
  keeps the quantum — glyphs, curves, diagonals, stroke outlines — and a path stating **several**
  rectangles is deliberately excluded, because taking them one at a time would trade this defect for
  item 5's seam. That seam has to be answered first.
- **The owner's session**: `tmp/pi.pdf`, for 628.
- **A push**: 630's CI fixes are green, but the fuzz repair and everything since has never faced a run.
