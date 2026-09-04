# 927 — A cost floor that a neighbour's load cannot move

Date: 2026-09-04.
ADRs: [0894](../adr/0894-a-cost-floor-that-counts-because-a-clock-here-is-a-lottery.md),
[0895](../adr/0895-what-the-counter-found-and-the-walks-it-does-not-reach-yet.md).
Touched: `crates/pdf-vfs/src/lib.rs`, `crates/pdf-vfs/src/worker.rs`,
`crates/pdf-vfs/src/cache.rs`, `crates/pdf-vfs/tests/a_face.rs`,
`crates/pdf-vfs/tests/read_corpus.rs`, `crates/pdf-vfs/tests/write_corpus.rs`,
`doc/todo/58`, `doc/todo/02`, `doc/conformance/ledger.toml`, `doc/questions/Q27`, two ADRs, this
file.
**No pixel moves**: no crate that draws changed.

A cost round, taken because `doc/todo/58` §5 said the sharpest thing missing from this crate was a
perf floor and said what its absence had cost — **a hundredfold regression that lived here for four
sessions with the entire gate sequence green** (ADR 0886). The general form, which is the round's
actual subject: this project gates correctness thoroughly and cost almost not at all.

## What was decided before any code

A wall clock is the obvious instrument and the wrong one. Session 922 measured why: two classes of
core, a factor of two on identical serial work at a load average under two, an unpinned spread of
100 % to 400 %, and three false failures already recorded in `doc/todo/02` §2. ADR 0884 answered
that for the launch path with five parts — nine fresh processes, a derived `taskset` list, two
calibration probes in the child, a `NOT JUDGED` mode and bands from forty-four runs — and none of
it transfers to a walk of a thousand documents through a rayon pool.

So the round counted instead. Trap 33's rule is that a property about how often something runs has
to count *that thing running*, and in this crate the expensive call is one seam: `Worker::ask`,
which every question crosses and which `Consenting` already wraps for the consent. The floor:

> **A generator runs once per subject per generation, and the only thing that may make it run
> again is the cache having stopped holding what it produced.**

`Vfs::questions().repeated <= Vfs::forgotten()`, per document. No band, no probe, no check file —
there is no clock in it, so a neighbouring round's load cannot move either side by one.

## What it found

Two things, and they are different kinds.

**A defect**, in the one layout row whose existence the document states rather than the layout.
Validating a path under `meta/xmp.xml` asked the worker for §14.3.2's *whole stream*, on every
listing of `/meta` and on every `stat` and `open` under it — nine questions cost eleven fetches on
the five-page annex. It is ADR 0886's shape one row along, and `Vfs::generated` was blind to it for
the same reason: the bytes were produced once. Fixed the way 0886 fixed its own — one function that
asks, the bytes cached under the path they belong to, and an absence remembered in the generation.

**A shortfall**, which the round deliberately did not fix: a refusal produces no bytes, so nothing
is cached and the next question runs the generator again. That is the whole of the first corpus
run's 36 "unexplained" documents, one or two questions each. `Questions` now counts the two apart,
and `doc/todo/58` §5 carries the shortfall with what fixing it would need.

## Trap 13, and the population lesson underneath it

Both floors were run against the defect. ADR 0886's own defect restored: the unit test fails **3
against 1**, the whole-layout floor **24 of 97 repeated**, naming five pages. This round's defect
restored: **11 against 1**, and **7 of 37**, naming `"metadata"`.

**The first of those proofs passed the first time it was run, and that is the finding.** The
whole-layout floor's two documents place no image between them, so the walk never entered
`images/NNNN/` — a floor whose population cannot reach the defect is trap 25 with a counter on it.
A third document is in that test because of it.

## What the floors read, and the title of this file

Over the whole population: `read_corpus` put **13 793** questions to its workers with **0** about a
subject already answered and **0** forgotten by the caches; `write_corpus`, whose five verb mounts
a document are measured by a `Drop` guard so that every early return is counted too, put **16 217**
with the same two zeros. The 51 in the read walk's fourth column are the refusal shortfall above.

The A/B came for free. The read walk ran twice on this tree — once beside a neighbour's gate
sequence, once inside this round's own — and the **wall clock was 200.3 s and 109.9 s** while the
**questions were 13 793 both times**, every column identical. That is the whole argument for
counting rather than timing, measured rather than asserted.

## What was left

The other seven corpus-scale gates have no cost floor, and ADR 0895 §3 surveys each: what the
counted invariant would be, and what it would cost. Three of them need no library change at all —
the counters exist and nothing asserts on them. `doc/questions/Q27` puts the ranking to the owner,
along with the question of whether a walk *owes* a cost floor the way it owes a population.
