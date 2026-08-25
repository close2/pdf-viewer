# 744 — The queue a census named, and nobody could order

741's owed item taken: *the three-measure "we are alone" list has never been read as a list*. It is
read now, and reading it needed one number the gate did not have. Parallel round, worktree `r744`,
branch `round-744`. **No pixel moves, no verdict moves and no list changes**: every one of the 962
non-agreeing per-page lines and every census figure is identical between the run before the round
and the run after it. ADR 0647 has the argument.

## What was missing, and why the census could not be opened

`doc/todo/00` step 1's *we are alone* is our distance from the **nearest** reference against the
distance between the **closest two** references. Session 518 took that by hand, in levels of 255,
and recorded 56 of 786 pages.

741 put the gate's two columns into comparable units and printed a count beneath the ranking. It had
to be taken in the **four**-measure unit, because that was the only unit the pair's number existed
in — and read that way the shape names **569 of the 804** complete ambiguous pages. That is
`doc/todo/12`'s differing fraction arriving as a signal rather than a signal.

So this round gave the closest pair a figure in `Distance`'s unit
(`consensus_missed_in_three_measures`), extracted the three-measure arithmetic `Distance::of`
carried inline so that one implementation answers both sides, and added
`rank_the_pages_we_are_alone_on` — both counts, the ten largest ratios, and two counts underneath
that say when the ratio is not measuring what its name says.

## What the list turned out to be

**48 of the 804 in three measures**, which is ADR 0643's predicted figure reproduced by the gate
that now prints it.

**All ten at the head are documented departures.** `issue11403_reduced.pdf` 9.06×,
`bug766086.pdf` 5.68×, `bug1743245.pdf` 4.13×, `freeculture.pdf` pages 315, 329, 322, 333 and 323
between 3.39× and 3.21×, `issue4260_reduced.pdf` 2.90× and `issue16224.pdf` 2.78× — each held by an
`AMBIGUOUS_*` group whose argument is the reason it is there. That is the corrected instrument
agreeing with this tree's own record, which is what a corrected instrument's first reading should
mostly be.

**The reading worth more than the list is about the ratio.** Neither column has a floor at 1. On
**31 of the 48** the closest pair sits inside all three bounds — the page is ambiguous on the
differing fraction alone — and on **22** our own nearest is inside them too, so the ratio is between
two numbers that agree with everybody and it ranks a page higher the more closely the references
agree. The head is the sharpest instance: `issue11403_reduced.pdf` is ours 0.51 over 0.06, and its
own verdict line reads `differing alone, 6.24%/5.00%` — **a page whose disagreement is invisible to
the three measures the list is computed in**. Both units have a blind spot and they are different
ones.

The sublist an opening round wants is the nine where we are outside a bound and the closest pair is
inside: `bug766086.pdf`, `freeculture.pdf` 315, 322, 323, 329 and 333, `issue16224.pdf`,
`endchar.pdf` and `issue12337.pdf`.

## The five book pages, measured rather than cited

Three ladders on `freeculture.pdf` page 315 — ours through `examples/render_at`, `poppler` through
`pdftoppm -cropbox`, `mupdf` through `mutool draw`:

```text
            1x        4x        8x
ours     11.8908   11.9540   11.9855
poppler  11.8704   11.9478   11.9592
mupdf    11.9611   11.9979   11.9914
```

All three converge, ours lies between the other two at every rung, and at 8× the three are within
**0.032 of 255**. On all five pages our 72-dpi ink is inside the references' own spread to 0.09 of
255. **Both references reproduce, to the three decimals it prints, the ladder session 233 recorded
in `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE` 511 rounds ago**; ours is 0.032 heavier at 1× and 0.010 at
8×, in the direction ADR 0418's round recorded for this population.

**What lifts them is the denominator, and it is trap 9 in a place nothing had priced it.** Over the
book's 321 compared pages, `poppler` and `mupdf` — the two voting references that share
`libfreetype.so.6`, where `ghostscript` links its own statically linked copy — are the closest pair
on **9 of the 11 pages that reach this list** and on **7 of the other 310**, and their own median
MAE is **724** over those 11 against **1760** over the rest. Trap 9 is a list of ways shared code
manufactures an agreement between references; in a ratio that agreement is the divisor, so the
same mechanism accuses us instead of excusing somebody.

## Measured

Three full oracle runs and §2's whole sequence twice, the second of them after the round's last
edit. `PDFREF_CACHE` on the shared warm cache at a **100% hit rate — 6707 reference renders from
disk and 0 produced**, so no reference renderer was spawned and no figure here measures another
program. Load ran from 0.5 to 45 across the round, which is what three parallel neighbours cost;
**no timing claim is made and none was needed**, and the verdicts are pixel arithmetic over cached
rasters.

§2's sequence, run whole and green: `fmt` clean, `clippy --workspace --all-targets` under
`RUSTFLAGS="-D warnings"` clean, `nextest` **2648 passed, 18 skipped**, doctests clean, the fuzz
check clean, and the corpus (**974 documents, 67 incomplete**), oracle, text extraction, both
censuses, dates, xmp, jpeg2000, quorra corpus, fixed documents (**40 checked, 0 absent**) and
conformance (**192 passed**) gates all green.

Sweeps: `--bin quoted` **170 figures read, 100 confirmed**, unchanged. `--bin unpriced` **93
failing bounds over 61 pages, 93 named, 0 not**, unchanged, and it still names `issue6069.pdf` as
the one page whose printed line cannot say what its verdict rests on. `--bin pointers` 131 absent
and `--bin quotations` 38 diverging, both unchanged. **`--bin overtaken` moved, 43 → 48, and the
whole of the movement is this round's own ADR**: 0647 names the pages of nine notes it does not
rewrite, which is that sweep's documented noise. The one note this round *did* rewrite,
`AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`, is not among them, because it cites 0647 — which is
`doc/todo/02` §4's rule working rather than a coincidence.

Not a fifth round (`tools/round.sh`) and no pixel moved, so §5's binaries were not rebuilt and
`doc/todo/00` step 7 was not re-run — neither has an input that changed, and the oracle's own
per-page lines say so. `tools/round.sh` reports this fresh worktree's `target/` as holding none of
§5's binaries, which is what a fresh worktree is.

## Changed

- `crates/pdf-model/tests/oracle.rs` — `outside_by_in_three_measures` (extracted from
  `Distance::of`), `consensus_missed_in_three_measures`, one field on `Examined`,
  `rank_the_pages_we_are_alone_on`, and a new section in `AMBIGUOUS_DENSE_TEXT_AT_BOOK_SIZE`.
- `doc/todo/00-ambiguous-bucket.md` — step 1's pointer, the third ranking, and the reading of the
  48.
- `doc/traps/oracle-and-references.md` — trap 9 gains the denominator.
- ADR 0647.
- No ledger row: the round implements no normative requirement and touches no clause.

## Owed

- **The nine.** This round opened five of them (one book) and named the other four; `bug766086.pdf`,
  `issue16224.pdf`, `endchar.pdf` and `issue12337.pdf` are each held by a group with an argument and
  none has been re-derived against this ranking.
- **Whether the ranking should require our own number to be outside a bound.** The counts say 22 of
  the 48 are ratios between two passes; nothing yet says whether dropping them would lose a page.
- **`doc/todo/12`'s 278 pages**, unchanged from 741 and the same subject from the bound's end.
- Unchanged from 741 and 729: `Distance` and `outside_the_bound` disagree about the contradicted
  pool with nothing stating which a round reaches for first; a *width* division and a *camp*
  division are treated alike; a voting reference whose raster is constant still votes;
  `freeculture.pdf` page 255; the owner's `git stash drop`.
