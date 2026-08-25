# 0643 — The fourth measure, asked of both pools and answered twice

**Status.** Accepted. Session 741.

Answers the question ADR 0636 left open and declines the change it looked like it was asking for.
**No pixel moves, no verdict moves, no list changes and no published figure moves**: the gate's
census and all 970 non-agreeing per-page lines are identical before and after, and `--bin quoted`
reads the same 170 figures and confirms the same 100.

## Context

`Distance::of` reduces a comparison to **three** ratios — mean, worst tile, structural similarity —
against the bounds the page was held to. The differing fraction, which is `Tolerance::accepts`'
fourth measure, is not among them.

ADR 0242 found the same absence one level down, in the per-page **line**, and fixed it there:
thirty of that session's sixty-eight contradicted pages printed a line on which every visible number
was inside the printed bound, because the bound that convicted them was the one nothing printed. It
declined to fold `Distance` into the new four-measure ratio, for a reason that has only got stronger
since — a hundred notes quote a `Distance` figure and `--bin quoted` (ADR 0495) now checks them
against the gate's own log, so `Distance` is a *published unit*.

ADR 0636 priced what that costs the **ordering** built on `Distance`, found that *the differing
fraction is the bound most of the contradicted pool fails on*, and handed the question on:

> `Distance` and this ratio disagree about the pool and nothing reconciles them.

The question this round was given is whether the fourth measure belongs in the ordering. The rule
for answering it is `doc/todo/00`'s: **a measure belongs in an ordering if it changes which page a
round should open next.** So it was measured rather than argued.

## The measurement

One oracle run over the whole corpus, `PDFREF_CACHE` on the shared warm cache, with every page's
four bounds and every one of its comparisons dumped and the two orderings computed side by side.
The populations are the run's own: 61 contradicted pages, 836 ambiguous, 804 of those complete.

### On the contradicted pool the fourth measure changes nothing anybody reads

`rank_the_contradicted` prints ten pages. **Under a four-measure `Distance` it prints the same ten,
in the same order, to the hundredth.** The differing fraction is the largest of the four ratios on 37
of the 61 pages — but never on the *nearest* comparison of a page near the head, which is what that
ranking is a fold over. Below rank ten it would lift nineteen more pages above 1.0, and that pool is
already ordered in four measures by `rank_the_contradicted_by_the_bound`, printed directly beneath
it since ADR 0636.

### On the ambiguous pool the fourth measure would replace the ordering, not sharpen it

The differing fraction is the largest of the four ratios on **762 of the 804** complete ambiguous
pages. A four-measure `Distance` would therefore order that bucket by one measure, and
`doc/todo/12`'s bound is the reason it cannot be that one. ADR 0243 established that
`TEXT_HEAVY::max_differing_fraction` sits **below** the spread of the implementations that set it,
rejecting 29.4% of reference pairs on text pages where its three siblings reject 0.0%, 1.2% and
0.5%. At page level over this pool that reads:

```text
  our differing fraction against the class bound, nearest reference   median 2.08
  the closest reference pair's, same bound, same pages                median 1.96
```

Six percent apart at the middle of the population, and on 222 of the 804 pages ours is the *smaller*
of the two. A measure on which we and the references fail the bound by nearly the same amount cannot
order a bucket by how far we sit from anybody.

The same fact stated as the reading `doc/todo/00` step 1 asks for — *we sit further from every
reference than the closest two sit from each other*, which that file reads as **we are alone**:

| the comparison, over the 804 complete ambiguous pages | names |
|---|---|
| ours in three measures against the pair's in three | **48** |
| ours in four measures against the pair's in four | **569** |

A signal that fires on seven pages in ten is not a signal.

### Calibrated against a figure taken by hand, in another unit, by a round that could not see this code

Trap 13. The five-hundred-and-eighteenth session took this reading over all 786 ambiguous pages of
the then corpus, **in levels of 255**, off the artefacts, and recorded that it is *true of 56 of the
786*. Over all 836 ambiguous pages of this run, in bounds:

- the **three**-measure reading names **58** — 6.9% against that session's 7.1%, two pages apart on a
  population fifty larger and in a different unit entirely;
- the **four**-measure reading names **583**, which is 70%.

The unit that reproduces an independently taken figure is the one in the code.

**And the mechanism was recorded once already, on a corpus that had no part in setting the bound.**
ADR 0543 took `doc/corpora/pdfbox`'s 63 ambiguous pages and wrote down that *59 of them fail a
differing-fraction bound that the closest two references also fail on every one of the 63*. That is
this round's median, on 63 pages instead of 804, written by a round that was diagnosing a population
rather than questioning a ranking.

### And the ordering the change would have moved prints nothing

`rank_the_undiagnosed` is the ranking that orders the ambiguous pool, and it filters to pages with no
diagnosis. `ambiguous_undiagnosed.txt` has been empty since ADR 0543, so **the list is zero rows
long in this run**, and re-ordering it would have changed nothing whatever. For the record of what it
*would* have promoted had it a population: the four-measure head of the complete ambiguous pool
brings in `issue6006.pdf` and `issue13520.pdf`, and every one of its ten is already held by name in
an `AMBIGUOUS_*` group.

## Decision

### 1. `Distance` stays at three measures

Not because its figures are quoted — that argument was already recorded and is about cost, not about
correctness — but because the four-measure unit is **worse at the job the ordering has**, on both
pools, measured. `Distance::of`'s doc comment says so now, so that the next round finds an answered
question rather than a standing debt.

### 2. The fourth measure is a *second number*, printed where the two columns needed it

The measurement turned up a defect of its own, and it is ADR 0242's shape in a place nobody had
looked: **`rank_the_manufactured_ambiguity` prints two numbers on one line and they are two different
instruments.** `Examined::consensus_missed_by` is `outside_by` — all four measures — and the `ours`
column beside it was `Distance::nearest`, which is three. The ranking's own doc comment and
`doc/todo/00`'s step 1 both ask a reader to compare them.

`jp2k-resetprob.pdf` page 1, the head of that list, is the plainest witness:

```text
  35.12 between them,  5.03 ours in three measures,  32.42 in four
```

Read as printed, the references disagree seven times more than we differ from the nearest of them.
In one unit the two numbers are eight percent apart. Over the pool, taking the printed columns as a
ratio names **13** of the 804 as pages we are alone on, where the like-for-like three-measure reading
names 48 and the four-measure one names 569 — so the mixed reading was not a conservative version of
either question but an answer to neither.

So `nearest_on_every_measure` is added beside `Distance`, the line prints ours in both units, and the
count under the list is taken in the pair's unit, which is the only one the two columns share.

## Consequences

- `nearest_on_every_measure` in `crates/pdf-model/tests/oracle.rs`; `Examined` gains one field;
  `rank_the_manufactured_ambiguity` prints the second column and a census line under the list.
  `Distance::of` and `outside_by` gain the answer in their doc comments.
- `doc/oracle-and-corpus.md` §3b, `doc/todo/12` and `doc/todo/00`.
- No ledger row moves: the round implements no normative requirement and touches no clause. The
  change is in a test target, so no pixel can move.
- **`Distance`'s figures are untouched**, which is the whole reason the fourth measure went beside it
  rather than into it: every note quoting one still quotes the number the gate prints.

## What this does not claim

That the differing fraction says nothing. It decides most of the contradicted pool's verdicts and
`rank_the_contradicted_by_the_bound` orders that pool by it, correctly, because there the ratio is
taken against a bound a consensus actually derived and applied. What this round establishes is
narrower and is about the *other* pool: on an ambiguous page no set judged anything, the bound is the
unwidened class floor, and against that floor the differing fraction separates us from the references
by six percent at the median.

## Owed

- **The three-measure "we are alone" reading is now checkable and has never been swept.** 58 pages of
  836, computed by the gate's own arithmetic rather than by hand in levels; `doc/todo/00`'s step 1
  says that shape is the one worth opening, and nobody has read the list as a list.
- **`doc/todo/12`'s bound now has a second population behind it.** ADR 0243 measured it on 2638
  reference *pairs*; this round measures it on 804 pages of *our* comparisons and gets the same
  answer, which is one more piece of the case that the number is wrong and one more thing the 278
  pages have to be argued against.
- Unchanged from 737: `Distance` and `outside_the_bound` disagree about the contradicted pool and
  nothing states which a round reaches for first; the pool is ordered and not yet read in that order.
