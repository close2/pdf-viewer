# ADR 0793 — A bound on the stack, not a guard against a population: `MAX_FORM_DEPTH` is 64, counts every nested content stream, and a tiling cell was counting none

Status: accepted. Session 874.
Clauses: ISO 32000-2 §7.8.2, §8.7.3.1, §8.10.1, §9.6.4 (Errata Collection 3, Issue #111),
§11.6.5.1, §C.2 Table C.1.
Code: `crates/pdf-model/src/content.rs` (`MAX_FORM_DEPTH`, `Interpreter::nesting`),
`crates/pdf-model/src/content/run.rs` (`Interpreter::run`, where the bound is asked),
`crates/pdf-model/src/content/{xobject,text,pattern,transparency,annotations}.rs` (the
`form_depth` parameter removed from every signature), `crates/pdf-model/examples/form_depth_cost.rs`
(the instrument), `tools/bounded.sh` (the incident below).
Tests: `crates/pdf-model/tests/hostile_budgets.rs::a_tiling_pattern_whose_cell_fills_with_itself_is_refused_by_name`,
`::a_form_and_a_tiling_cell_that_reach_each_other_are_refused_by_name`,
`::a_coloured_glyph_and_a_tiling_cell_that_reach_each_other_are_refused_by_name`,
`::a_chain_of_forms_the_witnesses_deep_draws_whole`,
`::the_sixty_fourth_nested_form_draws_and_the_sixty_fifth_is_refused_by_name`, and the two that
were there.
Takes `doc/todo/49`'s `MAX_FORM_DEPTH` item, opened by `doc/todo/03` section 39; amends ADR 0271's
row for it.

## Context

`MAX_FORM_DEPTH` was 16, and its comment argued the value from a population: every document of
two corpora that reached it was a form drawing itself, established by lifting the bound to 256 in
a scratch build and finding every witness still reached it (ADR 0271 over the SafeDocs crawl's
four, the eight-hundred-and-fifty-seventh session over the Mozilla tracker's seven). The
eight-hundred-and-seventy-first ran the same experiment over sixteen GHOSTSCRIPT-tracker witnesses
and found two that stopped — pdftk's stamps at 17–32 forms deep, Aspose.Pdf's at 33–64 — both drawn
blank at sixteen where `mupdf` draws the page. `doc/todo/49` then carried the constant as a
decision owed, with the method: measure what a level costs in stack, set the bound from that, and
re-run all twenty-seven witnesses at the candidate so that the claim about cycles is re-made rather
than inherited.

The question as put was *how deep*. Read against the code it was two questions with different
answers, and a third nobody had asked.

## What the standard says

Nothing that forbids depth, and nothing that forbids a cycle in a form. §C.2's Table C.1, in its
*Nested objects* row:

> As described in this PDF standard, many constructs can be nested including stitching
> functions, q / Q operators, XObjects, article threads, etc. However PDF processors may
> implement recursive algorithms which may cause issues for excessively nested constructs.

Annex C is informative; what the row states is that the depth is the processor's problem, and it
names the mechanism — a recursive algorithm — which is the thing this bound is about. §9.6.4, as
Errata Collection 3 inserts it below NOTE 1, is the one normative sentence on the subject and it is
about one of the five kinds:

> Implementations also need to avoid potential infinite recursion if a Type 3 glyph description
> refers to itself directly or indirectly. The result in all such cases is implementation-dependent.

§7.8.2 is what makes the five one thing: a form XObject, a tiling cell, a Type 3 glyph description
and an annotation appearance are content streams "packaged as sequences of instructions as
self-contained graphical elements", and a soft mask's group is a form (§11.6.5.1). Each is run by
`Interpreter::run` on the thread drawing the page, and each is one more frame of it.

## What was measured

`examples/form_depth_cost` generates a chain of nested streams of one kind — `F1 → F2 → … → Fn`,
the last filling a square — and for each of two depths bisects the smallest thread stack on which
`interpret` finishes, in a child process per probe because an overflow is an abort rather than an
error. The difference of the two thresholds over the difference of the depths is one level's
frames, with the page, the document and `interpret`'s own entry cancelled. Under
`[profile.release]`, over 48 levels (8 deep against 56), to within 4 KiB:

| kind | bytes of stack per level | 8 deep | 56 deep |
|---|---|---|---|
| form XObject (§8.10) | 3925 (3.8 KiB) | 56 KiB | 240 KiB |
| transparency group XObject (§11.6.6) | 5290 (5.2 KiB) | 68 KiB | 316 KiB |
| Type 3 glyph description (§9.6.4) | 9130 (8.9 KiB) | 96 KiB | 524 KiB |
| soft-mask group (§11.6.5.1), over 1–4 deep | 5461 (5.3 KiB) | 28 KiB at 1 | 44 KiB at 4 |
| tiling cell (§8.7.3.1) | see the third finding | | |

Sixteen against one, before any change, gave the same figures within the resolution — 4096, 5461
and 9284 — so the cost is linear and the constant it is measured against does not move it. Under
`[profile.dev]`, which is what `cargo nextest` runs the tests in, the same three are **3498, 4864
and 4949 bytes** — smaller, not larger: `opt-level = 1` inlines less into `run_reader`'s frame,
and the release figure is the one that binds because it is the larger and the one the viewer
runs.

**The thread this runs on is 2 MiB.** Nothing in this tree sizes a stack — not the viewer's
`page renderer` thread, not the confined worker's rayon pool, not a test harness thread — so every
one of them has `std`'s default, and the main thread's 8 MiB is the one stack `interpret` never
runs on. Sixty-four levels of the costliest kind are 570 KiB, plus about 30 KiB under the chain:
under a third of the stack, and the rest of it is the rasteriser's after `interpret` returns.
One hundred and twenty-eight would be over half, and the two witnesses want at most sixty-four.
So: **64**, and the whole argument for the number is the row above times the stack it has to fit
in. It is not derived from the witnesses; they are what it has to draw.

## Three findings, one of which is the decision

**1. Twenty-five of the twenty-seven witnesses were never cycles — the experiment measured the
instrument.** `run_cell` ran a tiling cell's stream at `MAX_FORM_DEPTH - 1`, a figure chosen when
patterns were first drawn (`9efe4406`) so that a cell could hold one form. Lifting the bound to
256 lifted that starting point to 255 with it, so a cell holding *two* levels of forms reported the
bound at sixteen, at 256, and at any value a scratch build could name — and "still reaches it
lifted sixteenfold" was true of a document that nests forms three deep inside a pattern. Run at 64
with the counter below, the crawl's four, the Mozilla tracker's seven and fourteen of the
GHOSTSCRIPT sixteen **draw whole and report nothing**; the two GHOSTSCRIPT nestings draw whole; and
`GHOSTSCRIPT-698226-0.pdf` and `GHOSTSCRIPT-700301-0.pdf` report the bound at 64 and at 256 with the
same command counts, which is what a cycle looks like. ADR 0271's row — "all four are cycles … no.
A cycle exhausts the stack" — was right about what the bound prevents and wrong about who reached
it, and the eleven-document claim in the constant's own comment was the instrument's.

**2. A cycle guard by identity was considered and declined.** The obvious construction is a chain
of the object identities in flight, refusing a stream already on it before it runs — cheap,
immediate, and a report that could say *cycle* rather than *too deep*. It is wrong on finite
files. What a content stream invokes depends on the state it inherits: a form that fills a path
fills it with the *current* colour, and if that colour is a tiling pattern whose cell draws the
same form in a flat colour, the form appears twice on the chain and the file ends — a shared logo
form drawn on the page under a pattern fill whose cell is the logo in black is not absurd. A
glyph shown by a form under one font, whose description shows the form under another, is the same
shape. Both are conforming, both terminate, and both would be refused by name. A key of identity
*and* the state that selects what a stream invokes is the whole graphics state, which is the
thing a bound exists to avoid comparing. So the standard's own framing is kept: a cycle and a deep
nesting are the same thing to this bound, a chain of frames, and what tells them apart is only
that a cycle reaches any bound at all. The report stays `LimitReached { limit: "MAX_FORM_DEPTH" }`,
whose condition is exactly what it says.

**3. Until this session the bound did not hold, and the measuring example found it on its first
run.** With cells starting one below the bound, a pattern reached from a pattern started counting
again from there, so nothing bounded the nesting of cells at all. A pattern whose cell fills with
itself, a form filling with a pattern whose cell draws the form, and a `d0` glyph description
doing the same through a pattern — three seven-object files — each recursed until the guard page
aborted the process: `fatal runtime error: stack overflow`, under `tools/bounded.sh`, in under a
second. The corpus's `ContentStreamCycleType3insideType3.pdf` is the third shape with a `d1` glyph
and survived only because §8.6.8 makes a `d1` description ignore the `scn` that would re-enter.
This is the case principle 3 names, and it was open for as long as patterns have been drawn.

## Decision

- **One counter, in one place.** `Interpreter::nesting` is raised by `Interpreter::run` on the way
  into any nested content stream and lowered on the way out, and `run` refuses at
  `MAX_FORM_DEPTH` before reading a token. The `form_depth` parameter that was threaded through
  eleven signatures — and that a cell set to a constant, a soft mask's group to zero and an
  appearance to one — is gone, so no kind of nested stream can be run without the bound and no
  call site can choose its own starting point. That is the fix for finding 3 and the reason the
  check is not at the call sites.
- **`MAX_FORM_DEPTH` is 64**, argued from the table above and the 2 MiB it has to fit in, and its
  comment says so; the name is kept because the reports and the surveys have counted by it since
  the bound existed, and the comment says what it now bounds.
- **`MAX_SOFT_MASK_DEPTH` stays at four beside it.** It bounds a different cost — each level is a
  whole group's commands — and the mask's group now counts against both.
- **`examples/form_depth_cost` is kept** as the instrument this ADR's table came from, with
  `--write DEPTH PATH` so that a chain can be handed to `open_one` or a reference renderer. Its
  refusal criterion is a `LimitReached` report rather than an empty display list, because a soft
  mask refused at `MAX_SOFT_MASK_DEPTH` still lets the page's own square through.
- **`tests/hostile_budgets.rs`** holds the three cycles through a cell, a chain of forty (between
  the witnesses' depths) drawing whole, and sixty-four drawing against sixty-five refused — the
  last so that a change to the value in either direction fails here rather than in a corpus.

## What the example also found, and this ADR only records

A chain of tiling patterns each filling with the next multiplies the display list per level: a
fill strictly inside one cell still tiles **nine** — the span takes a neighbour on each side — so
the chain is 9ⁿ commands, 6561 at four deep, 531 441 at six, and at eight **8 503 056 commands and
2 GiB** before `MAX_OPERATIONS` stops it (`open_one` on `form_depth_cost --pattern --write`). It
is not the stack — the example reports the abort as an overflow only because a child that dies of
anything is not a child that drew — and it is not this bound's, since an eight-deep chain is well
inside sixty-four. It is `doc/todo/49`'s standing item, that `MAX_TILES` and `MAX_OPERATIONS`
bound counts where they mean to bound work, with a witness of its own shape: the count that stops
it is four million operators and the cost of four million copied commands is two gibibytes. That
file carries it; the tiling-cell row of the table above is empty for this reason, and the pattern
kind's per-level stack cost is measurable once a chain of one tile a level can be stated.

**And the raise unmasked it on a corpus document, which is the one fix this ADR makes outside the
bound.** `ContentStreamCycleType3insideType3.pdf` — a `d1` glyph whose pattern's cell shows the
glyph — was refused at sixteen the moment its cell ran, because the cell started one below the
bound. At sixty-four the cycle is entered sixty-four levels deep, every level is nine copies of the
one below it, and `repeat_cell` charged each copy to `MAX_OPERATIONS` *after* making it: the
innermost tiling stopped at four million commands and every enclosing tiling copied that list nine
times over. The corpus gate died of a 1.9 GB allocation under an 8 GiB `RLIMIT_DATA`, and the
document alone, walked without a bound — which this round should not have done, and says so in
its history file — reached 25 GB and a minute. The budget is asked **before** the copy now:
`self.operations + cell.len() > MAX_OPERATIONS` refuses the copy that would cross it, so the list
is bounded by the budget plus one cell. The document costs 3 995 603 commands, 1.46 GiB and 3.8 s
under that — the whole operator budget spent on a cycle the old cell depth refused for nothing,
which is the honest price of a bound that holds for the two real nestings, and is inside the
confined worker's ceiling and inside what ADR 0271 already recorded as `MAX_OPERATIONS`'s worst
document (1.57 GB). `tests/hostile_budgets.rs::a_marking_cycle_through_a_tiling_cell_stays_inside_the_operator_budget`
asserts the count.

## The incident, which this round owns

Naming the Mozilla tracker's seven witnesses needed a survey over its 6835 documents, and this
round launched it as `tools/bounded.sh --data 32 -- safedocs survey …` in the background — the whole
32 GiB walk budget for one 24-thread process, with no `--tree` ceiling — beside the owner's
desktop, the Claude process, `sccache` and two other rounds' gates and builds. The user slice's
memory peaked at 61.09 GB of 61.9; every shell call of this round and its neighbour's stalled from
09:05; the survey was killed at 09:07:23; and at 09:08:04 the Claude process aborted, by its own
`abort()` rather than by `oomd` or the kernel (`oom_kill` is 0 in every cgroup). `RLIMIT_DATA` is
per process and 32 GiB was sized for a machine running one walk, not three rounds. The owner's
four rules are now in `doc/environment.md` and in `tools/bounded.sh`'s header with this timeline,
and the script refuses `--data` above 12 GiB without `--tree` and defaults `--tree` to 12 where
none is given. The survey that named the seven was re-run at `--shards 2` and its findings on the
bound stand; its 256 "incomplete" verdicts whose only fault was a stale sandbox worker measured
nothing and were not used.

## Consequences

- The two GHOSTSCRIPT nestings draw. Twenty-three documents across three corpora that reported
  this bound report nothing. Two cycles are refused by name, as before.
- Three stack overflows reachable from a seven-object file are refusals by name.
- `doc/todo/49`'s `MAX_FORM_DEPTH` row is closed and its "counts the wrong quantity" item gains the
  nested-pattern witness. `doc/todo/03` section 39's "fourteen are cycles" is corrected there.
- Ledger rows §7.8.2, §8.7.3.1, §8.10.1, §9.6.4 and §11.6.5.1 say where the bound is asked and why.
- **A lesson for `doc/traps/instruments-and-reports.md`**, as trap 29: a bound lifted in a scratch
  build is lifted only where the code reads the constant, and a site that *derives* its own number
  from it — `MAX_FORM_DEPTH - 1` — is lifted with it, so "still reaches the lifted bound" can be a
  property of the derivation rather than of the file. Calibrate a lifting experiment the way trap
  13 calibrates a sweep: with one document known to be finite and deep, which must stop.
