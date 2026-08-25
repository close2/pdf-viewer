# 741 — The fourth measure asked, and declined

737's question taken and answered with numbers rather than with code. Parallel round, worktree
`r741`, branch `round-741`. **No pixel moves, no verdict moves, no list changes and no published
figure moves**: the census and all 970 non-agreeing per-page lines are identical between the run
before the round and the run after it, and `--bin quoted` reads the same 170 figures and confirms the
same 100. ADR 0643 has the argument.

## The question, and the rule for answering it

`Distance::of` keeps three of `Tolerance::accepts`' four measures, and the differing fraction — the
bound most of the contradicted pool fails on — is the one left out. ADR 0636 priced that for the
ranking built on it and handed the question on as unreconciled. `doc/todo/00`'s rule decides it:
**a measure belongs in an ordering if it changes which page a round should open next.**

So it was measured on both pools that `Distance` orders, from one oracle run with every page's four
bounds and every one of its comparisons dumped.

## What the measurement said

**On the contradicted pool the fourth measure changes nothing anybody reads.** `rank_the_contradicted`
prints ten pages, and under a four-measure `Distance` it prints the same ten, in the same order, to
the hundredth. The differing fraction is the largest of the four ratios on 37 of the 61 pages, but
never on the *nearest* comparison of a page near the head — which is what that ranking folds over.
Below rank ten it lifts nineteen more pages above 1.0, and that pool is already ordered in four
measures by `rank_the_contradicted_by_the_bound`, printed directly beneath it since 737.

**On the ambiguous pool it would replace the ordering rather than sharpen it.** The differing fraction
is the largest of the four ratios on 762 of the 804 complete ambiguous pages, and `doc/todo/12`'s
bound is why that disqualifies it: our differing fraction sits at a median **2.08** times the class
floor against the closest reference pair's **1.96**, with ours the smaller of the two on 222 of the
804. Read as *we are alone*, three measures name **48** of those pages and four name **569**.

**Calibrated against a figure taken by hand, in another unit, by a round that could not see this
code** (trap 13). Session 518 took that reading over 786 ambiguous pages in levels of 255 and recorded
*56 of the 786*. In bounds over all 836 of this run: the three-measure reading names **58** — 6.9%
against 7.1% — and the four-measure reading names **583**, which is 70%. ADR 0543's own sentence about
the pdfbox 63 says the mechanism a third time.

**And the ordering the change would have moved prints nothing.** `rank_the_undiagnosed` filters to
pages with no diagnosis, `ambiguous_undiagnosed.txt` has been empty since ADR 0543, so the list is
zero rows long in this run. Its four-measure head would have brought in `issue6006.pdf` and
`issue13520.pdf`, and every one of its ten is already held by name in an `AMBIGUOUS_*` group.

## So: declined, and one thing fixed that the measurement turned up

`Distance` stays at three measures — not because its figures are quoted, which is a cost rather than
a reason, but because the four-measure unit is worse at the ordering's job on both pools, measured.

What the measuring found instead is ADR 0242's defect in a place nobody had looked:
**`rank_the_manufactured_ambiguity` prints two numbers on one line and they are two different
instruments.** `consensus_missed_by` is `outside_by`, all four bounds; the `ours` column beside it was
`Distance::nearest`, three of them. The ranking's own comment and `doc/todo/00` step 1 both ask a
reader to compare them. On the head page that reads

```text
  35.12 between them,  5.03 ours in three measures,  32.42 in four
```

— as printed before this round, the references appeared to disagree seven times more than we differ
from the nearest of them; in one unit the two numbers are eight percent apart. Over the pool, taking
the printed columns as a ratio names **13** of the 804 where either single unit names 48 or 569, so
the mixed reading was not a conservative version of either question but an answer to neither.

`nearest_on_every_measure` is added beside `Distance`, the line prints ours in both units, and the
count under the list is taken in the pair's unit, which is the only one the two columns share.

**And one note was making that comparison in prose.** `AMBIGUOUS_IMAGE_REDUCTION` set the 35.12
against "our own 5.03 from the nearest"; corrected, with the reason and ADR 0643 cited. Its conclusion
is untouched and that is the point — it never rested on the contrast but on the pairwise table above
it, which is one instrument over all ten pairs. `--bin overtaken` found this: the sweep went 43 → 44
when the ADR landed and back to 43 when the note was corrected, which is the sweep doing exactly what
it is for.

## Measured

Four full oracle runs, `PDFREF_CACHE` on the shared warm cache at a **100% hit rate — 6707 reference
renders from disk and 0 produced**, so no reference renderer was spawned and no figure here measures
another program. Load ran from 2.0 to 12.5 across the round, which is what three parallel neighbours
cost; **no timing claim is made and none was needed**, and the verdicts are pixel arithmetic over
cached rasters.

The change is `crates/pdf-model/tests/oracle.rs` — a test target, no library code, so no pixel can
move — plus four documents and an ADR. §2's sequence was run whole anyway: `fmt` clean, `clippy
--workspace --all-targets` under `RUSTFLAGS="-D warnings"` clean, `nextest` **2638 passed, 18
skipped**, doctests clean, the fuzz check clean, and the corpus, oracle, text extraction, both
censuses, dates, xmp, jpeg2000, quorra corpus, fixed documents and conformance (**192 passed**) gates
all green.

Sweeps: `--bin quoted` **170 figures read, 100 confirmed**, unchanged, and no hit on the note this
round corrected — the 5.03 it quotes is still what the gate prints, in the column now labelled with
its unit. `--bin unpriced` **93 failing bounds over 61 pages, 93 named, 0 not**, unchanged, and it
still names `issue6069.pdf` as the one page whose printed line cannot say what its verdict rests on.
`--bin overtaken` **43**, unchanged after the note correction. `--bin pointers` 131 absent, unchanged;
`--bin quotations` 38 diverging, unchanged.

Not a fifth round (`tools/round.sh`), no pixel moved, so §5's binaries were not rebuilt and
`doc/todo/00` step 7 was not re-run — neither has an input that changed.

## Changed

- `crates/pdf-model/tests/oracle.rs` — `nearest_on_every_measure`; `Examined` gains one field;
  `rank_the_manufactured_ambiguity` prints the second column and a census line; doc comments on
  `Distance::of`, `outside_by` and `AMBIGUOUS_IMAGE_REDUCTION`.
- `doc/oracle-and-corpus.md` §3b, `doc/todo/00`, `doc/todo/12`, `doc/habits.md`'s judging section.
- ADR 0643.
- No ledger row: the round implements no normative requirement and touches no clause.

## Owed

- **The three-measure "we are alone" list has never been read as a list.** 58 pages of 836, and
  `doc/todo/00`'s step 1 says that shape is the one worth opening. Until this round it could only be
  taken by hand in levels of 255.
- **`doc/todo/12`'s bound now has a second population behind it and the 278 pages are still owed.**
  ADR 0243 measured the bound on 2638 reference pairs; this round measures it on 804 pages of our own
  comparisons and gets the same answer.
- Unchanged from 737: `Distance` and `outside_the_bound` disagree about the contradicted pool and
  nothing states which a round reaches for first; the pool is ordered and not yet read in that order.
- Unchanged from 729: a *width* division and a *camp* division are treated alike; `unpriced` cannot
  tell a bound named from a bound accounted for; a voting reference whose raster is constant still
  votes; `freeculture.pdf` page 255; the owner's `git stash drop`.
