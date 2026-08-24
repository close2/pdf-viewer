# ADR 0606 — The bound a note owes, asked of the whole pool

Status: accepted, 2026-08-25. Session 722. Adds `doc/todo/01`'s **twenty-first sweep**,
`--bin unpriced`, the sixteenth to be a program; rewrites two `CONTRADICTED_*` notes; fixes a
defect it found in the nineteenth and twentieth sweeps; amends two ledger rows.
**No pixel moves and no list changes.**

## The criterion, and why it is the sixth one and not a seventh

ADR 0497 warned that six rounds had each invented and spent one criterion for choosing a
contradicted group — closed form, name, member coverage, clause count, mechanism, sufficiency —
and that "[t]hat pattern cannot continue much further". Session 672 answered its own warning:
*the criterion a next round should reach for is not a seventh — it is the sixth pointed at those
eight.* Sessions 675 and 680 spent it on the eight, and every one came out defensible.

This round points the same criterion at the **pool**: all thirteen non-empty lists, every
contradicted page. What makes that a different job rather than a bigger one is that the sixth
criterion has a *precondition* nobody could evaluate at scale — you have to know which of the
gate's four bounds each page actually fails. Five rounds recorded that in the same words:

> Nothing links a group's note to *which* bound the gate fails its pages on. All thirteen
> diagnoses here began by reading that off a log by hand, and a note can go on explaining a mean
> while the page fails on a differing fraction for as long as nobody looks. `--bin quoted` checks
> a figure a note quotes; it cannot ask for one that is missing.

Sessions 489, 668, 672, 675 and 680. So the round is the criterion *and* the instrument, because
without the instrument the criterion is thirteen more log readings and the sixth debt.

## The sweep

**Discriminator**: a measure the gate fails one of a note's own pages on, in a verdict of
`CONTRADICTED`, that the note's prose never names.

Both sides are this project's own output, which is what the twentieth sweep established as
workable. The right-hand side is free: the oracle already prints all four measures beside all four
bounds for every page it does not call agreement, so *which bound fails* is `Tolerance::accepts`'
three ceilings and one floor evaluated on a line the round has already run. Nothing rasterises.

**The population is that one verdict and no other**, and that is trap 11 rather than tidiness. On
an `ambiguous` page no two references agreed, so the bound printed beside them decided nothing;
asking a note to account for it would manufacture a debt out of a line's shape. `not comparable`,
`no render` and the two geometry verdicts are reached without a comparison at all.

**Three rungs**, closest first, and the first is the shape the criterion was written for:

1. the note names measures and not one of them is a measure its pages fail — a note arguing about
   a mean over a page that fails on the differing fraction. `CONTRADICTED_GLYPH_EDGES` stood on
   that exact sentence for three hundred sessions (ADR 0242);
2. the note names no measure at all;
3. the note names one failing measure and misses another.

**Word presence, not `quoted`'s word-plus-figure**, and the difference is the point: *"all three
fail on mean and structural similarity"* names both bounds and quotes neither, which is exactly
what this sweep is looking for. The one place it guesses is that `mean` is also a verb;
`NOT_A_MEASURE` is the stated exclusion, its residue is a hit not printed rather than one
invented, and that is the direction trap 11 asks a report to err in.

**Calibrated per trap 13 against a live defect rather than a plant**, which is stronger: the
finding below was in the tree when the sweep was written, came out at rank 1, and the run is
silent once the note was written.

## What it found, and the third one is about the instruments

**`CONTRADICTED_TIGHT_CONSENSUS` names one measure in a hundred and sixty lines, and it is one of
its three pages'.** The worst tile at 6.73 against 6.04 is `issue7891_bc1.pdf`'s.
`colors.pdf` pages 1 and 2 fail on **structural similarity and on nothing else** — page 1 prints
mean 0.21 of 1.00, worst tile 2.63 of 5.00 and differing 0.50% of 1.00% — and the note's only
account of them is a table of four decimals under the words *bound* and *ours*, with no unit
anywhere near it. That is a note whose own opening paragraph condemns it: it says of a *different*
page that "between them sat no account of the metric that fails".

The group's sentence — *a bound no analytic-coverage renderer meets on a page that is nothing but
edges* — was argued from mean distances to a closed form, so it is now argued in the metric that
fails. `Tolerance::widened_to` scales the distance from 1.0, so the bound is
`1 − 2 × (1 − ssim(poppler, ghostscript))`, and the pair's 0.99431 and 0.99201 give the gate's
0.98862 and 0.98402 exactly. Every renderer against the pair, from the run's own artefacts:

```text
                          page 1     page 2
  poppler <-> ghostscript 0.99431    0.99201   the pair: it sets the bound
  ours                    0.98786    0.98024   fails
  hayro                   0.98772    0.98011   fails, by more than ours
  mupdf                   0.98739    0.97943   fails, by more than either
```

**Two renderers that are not this tree fail the same bound, and both fail it by more than we do.**
One is an independent C interpreter and one a separate Rust one; neither is a party to how we
round an edge. `ours ↔ hayro` is 0.99999 with a worst pixel of two levels, so the ranking of the
four is the ranking of how much anti-aliasing each does, and the three that converge on the
geometry are held to twice the distance between the two that do not. That is the sixth criterion
answered in the units the gate uses, with about as close to a control as trap 12's shape admits:
taking *us* out of the room does not rescue the bound.

**`issue6069.pdf` page 1's verdict is six channels of eighty thousand.** The sweep asked which
bound fails it and got *none*: the gate prints `differing 6.55%` against `bound … differing
6.55%`, identical at the two decimals it writes. At full precision `poppler` against `mupdf` is
3.2738% of channels, so the bound is 6.5475%, and ours against `poppler` is 6.5550% — on a
400 × 50 raster, **5244 differing channels against an allowance of 5238**. The page stays
contradicted, because the arithmetic is the arithmetic; what is worth recording is that **a page's
own line can stop being able to say what its verdict rests on**, and no instrument but this one
was placed to notice. `CONTRADICTED_SUBSTITUTED_FONT`'s row for it read 6.62% until now and the
ablation is untouched — the embedded face takes the page to 5.97% and inside.

**And sixty-nine page names were invisible to the nineteenth and twentieth sweeps.**
`overtaken::documents_in` rejects a `.pdf` token preceded by `/`, with the reason written beside
it: *"`doc/ISO_32000-2_sponsored_EC3.pdf` is the standard, not a page"*. True when written, and
overtaken one round later by ADR 0541, which gave every page of a submodule corpus its corpus's
label — `pdfbox/attachment.pdf page 1` — **precisely because** three of those documents share a
bare file name with one of the 974 and only two of the three share their bytes. So the label is
the identity and not a path, and the rule silently emptied `Note::pages` of every submodule-corpus
page for `overtaken`, for `quoted` and for this sweep. The exclusion was never needed either:
`Corpus` is built from the lists' own members, so a `.pdf` token no list holds is narrowed away
whatever its shape. Taking the separator into the name and letting `Corpus::narrow` do the one job
it always did, measured on the same oracle log:

| | before | after |
|---|---|---|
| `overtaken` vocabulary | 320 documents | **340** |
| `quoted` figures confirmed / unanchored | 86 / 21 | **91 / 13** |
| `unpriced` contradicted pages held by no note | 5 | **0** |

**A rule written to exclude one file excluded a naming convention that did not exist yet**, and
that is this round's transferable lesson rather than the two notes. It is the nineteenth sweep's
own subject — *a sentence that was true when written and that nothing pointed at when the tree
moved under it* — arriving in the nineteenth sweep's own code.

## What was not done, and why

**Nothing was moved off a list and no verdict changed.** The question this round asked is whether
the pool's exemptions are accounted for in the units the gate uses, and the answer on every page
of it is now yes; that is not the same as an exemption being *right*, and the sweep says so in
its own report — it asks whether the bound is named, never whether the argument persuades. Session
651's rule holds: incrementing a tally you cannot defend is not a round.

**The sweep is not a gate**, for the nineteenth and twentieth's reason. A note may name a bound
and argue the failure is not ours, a group's pages need not fail on one bound, and every hit is a
sentence a person has to read. It ranks, it names the page and the measure, and it exits zero.

## Owed

- **Nothing ranks the *pool* by how far outside its bound each page sits.** `rank_the_contradicted`
  orders by distance from the nearest reference and ADR 0349 left the other ordering unbuilt; the
  arithmetic for it is `outside_by`, which the gate already computes for every page.
- **`unpriced` cannot tell a bound *named* from a bound *accounted for*.** It found the thirteen
  notes' vocabulary complete; whether each mechanism owns its margin is still the sixth criterion
  done by hand, and it is done for eleven of the thirteen (ADRs 0499, 0510, and this one).
- Unchanged from 680: a voting reference whose raster is constant still votes; `freeculture.pdf`
  page 255; the owner's `git stash drop`.
