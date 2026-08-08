# 0243 — A bound below the spread it was measured on

Status: accepted, in the four-hundred-and-seventh session.
Supersedes nothing. Touches `tools/pdfref/`, `crates/raster-compare/` and
`crates/pdf-model/tests/oracle.rs` — comments, one report line and one new instrument. **No
rendering code, no pixel, and no verdict.**

## The decision

**`Tolerance::TEXT_HEAVY::max_differing_fraction` stays at 0.05, and it stays as a documented
departure from its own derivation rather than as a number nobody had checked.** The derivation
now exists, is reproducible, and says the bound is too tight; the reasons for not acting on it
are measured and written down here.

Three things change beside it, and none of them moves a verdict:

- **A new instrument**, `oracle.rs`'s `the_fixed_bounds_against_the_references_own_spread`,
  which re-derives all eight fixed bounds from the corpus — every reference pair, split by
  tolerance class and by whether the pair crosses the hinting boundary, each measure taken over
  the pairs the *other three* bounds admit.
- **`report::summarise`'s reference-against-reference line prints four measures where it printed
  three.** The differing fraction was missing from the one line in the tree that shows what a
  fixed bound is derived from — ADR 0242's defect one file earlier in the pipeline.
- **Three comments corrected**, below.

## The measurement

`the_fixed_bounds_against_the_references_own_spread`, over the oracle's own 1794 pages and
**9898 reference pairs**, `poppler` 26.07.0, `mupdf` 1.28.0, `ghostscript` 10.07.1 and `hayro`.
Each measure's distribution is taken over the pairs that the class's other three bounds admit —
which is `Tolerance::VECTOR`'s own stated method ("the pages where every reference pair already
falls inside the three pixel bounds above have a structural similarity of 0.9971 at worst") and
is what stops the measurement from being circular: a bound measured over the pairs it already
admits returns the bound.

**Text pages, `Tolerance::TEXT_HEAVY`, the three independent references against each other:**

| bound | its value | pairs | median | p90 | p99 | max | pairs it rejects |
|---|---|---|---|---|---|---|---|
| mean | 5.00 | 1862 | 0.3189 | 1.9091 | 3.5450 | 4.6117 | **0.0%** |
| worst tile | 40.00 | 1884 | 9.0952 | 23.9790 | 41.1076 | 100.4004 | **1.2%** |
| **differing fraction** | **5.00%** | **2638** | **1.6871%** | **10.3849%** | **12.0199%** | **28.0398%** | **29.4%** |
| structural similarity, as 1 − ssim | 0.1000 | 1871 | 0.0048 | 0.0356 | 0.0835 | 0.1508 | **0.5%** |

**The same pages, `hayro` against one of the three** — two implementations sharing no code with
each other, one of which grid-fits glyphs through `libfreetype` and one of which does not:

| bound | pairs | median | p90 | p99 | max | pairs it rejects |
|---|---|---|---|---|---|---|
| mean | 1460 | 0.2764 | 1.4948 | 2.8708 | 5.7194 | 0.1% |
| worst tile | 1470 | 5.9590 | 18.5742 | 37.1814 | 101.2500 | 0.7% |
| **differing fraction** | **2436** | **3.4200%** | **11.2357%** | **16.0024%** | **48.2066%** | **40.1%** |
| structural similarity, as 1 − ssim | 1464 | 0.0031 | 0.0218 | 0.0617 | 0.1475 | 0.3% |

**Vector pages, `Tolerance::VECTOR`, the three references against each other** — 1121 pairs,
and the control:

| bound | its value | pairs | p99 | max | pairs it rejects |
|---|---|---|---|---|---|
| mean | 1.00 | 492 | 0.5144 | 0.6107 | 0.0% |
| worst tile | 5.00 | 545 | 18.2500 | 55.7500 | 9.7% |
| differing fraction | 1.00% | 506 | 1.3638% | 5.7143% | **2.8%** |
| structural similarity, as 1 − ssim | 0.0100 | 509 | 0.0176 | 0.0700 | 3.3% |

## What the measurement says

**One bound of the eight sits below its own references' spread, and it is the one 38 of the
gate's 68 contradicted pages fail on and nothing else — 37 of them text pages held to this bound,
the thirty-eighth `transparent.pdf` against `VECTOR`'s.** On text pages `max_differing_fraction`
rejects 29.4% of the reference pairs that agree by every other measure; its three siblings
reject 0.0%, 1.2% and 0.5%. The *same measure* on vector pages rejects 2.8% against its
siblings' 0.0%, 9.7% and 3.3% — so this is not a property of the metric, and not an argument
that a channel count is a bad thing to bound. It is one number in one class.

**The sentence that claims to derive it names another measure's number.** `TEXT_HEAVY`'s comment
reads "[m]easured on the specification PDFs in `doc/`: the three references disagree with *each
other* at a worst tile of 26 to 28, with 2.7% of pixels differing". Re-run on exactly that
population — the 14 specification PDFs' first pages, **42 reference pairs** — the worst tile
reproduces to the digit: p90 **26.72**, max **28.17**. The differing fraction on those same pairs
is median 3.11%, p90 4.99%, **max 5.14%**, and **11.9% of them are already outside the 5.00%**.
What *is* 2.7 on that population is `mean_error`'s maximum, 2.7355. The bound of the four that
names no derivation of its own was given another measure's, and the population it was taken from
exceeds it.

**And the pages that fail it are inside the reference population.** Our differing fraction on the
21 pages of `CONTRADICTED_GLYPH_EDGES` runs 5.07% to 10.58%, between that population's median
(1.69%) and its 99th percentile (12.02%). Trap 12 in one line: the verdict on those pages is a
statement about the consensus pair being unusually close, not about the marks — which the ink
table and the twice-drawn glyph in that group already said by two other routes, without a bound.

## Why the bound is not moved

**Because it does two jobs, and the derivation speaks to one of them.** `Tolerance::accepts`
decides whether two references form a consensus at all, and the same numbers floor the per-page
bound `widened_to` derives. Raising `max_differing_fraction` to the 99th percentile of the
reference spread, 0.12, was run over the whole corpus rather than reasoned about:

| | before | at 0.12 |
|---|---|---|
| agrees | 905 | 1121 |
| **contradicted** | **68** | **309** |
| ambiguous | 786 | 329 |

**457 pages leave `ambiguous` and 278 of them arrive newly contradicted**, against 37 of the 38
leaving. The dominant effect of loosening this bound is not leniency towards us — it is that
pairs of references which do not presently agree start forming a consensus, and that consensus
then contradicts us on 278 pages nobody has looked at. A change of that size is a programme of
work with 278 diagnoses in it, and adopting the number without them would be the count that
improves without a picture.

**And the population that would justify loosening our side alone is not admissible evidence about
us.** The obvious narrower move — keep 5% for consensus, floor our own judgement at 12% — needs
an argument that the pair *including us* is legitimately further apart than a pair of references,
and the measurement supplies one: across the hinting boundary the median differing fraction
doubles, 1.69% to 3.42%. But the only renderer on the far side of that boundary is `hayro`, which
shares `skrifa` with this tree. Widening our own bound because `hayro` sits far from `poppler`
would forgive whatever the two of us get wrong together — the circularity `Reference::
independence` exists to prevent, and it is the same circularity whether it reaches a verdict
through a vote or through a bound. `widened_to`'s comment has said for four hundred sessions that
what would justify a change is "a measurement of how far a *fourth* independent rasteriser sits
from the three, which nobody has". This session took the nearest thing available and it is still
not that renderer.

**Loosening a gate to make contradictions disappear is the move this project forbids itself**, and
37 pages leaving a list because a number was raised is exactly what that looks like from outside.

## What was corrected

- **`Comparison::differing_fraction` counts channels, not pixels** — the denominator in
  `compare_with_tile` is `width × height × 4`, alpha included. Its own doc comment said "fraction
  of pixels" and `TEXT_HEAVY`'s said "2.7% of pixels differing". A pixel whose red alone moved
  contributes a quarter of what "one differing pixel" suggests, so every figure of this kind in
  the tree reads lower than a per-pixel one would.
- **It is not a count of channels that moved *at all*.** `CONTRADICTED_GLYPH_EDGES`'s comment said
  so in the four-hundred-and-sixth session; the threshold is `JUST_NOTICEABLE = 4`, so a channel
  that moved by four levels or fewer is not counted. The group's diagnosis is unaffected and
  slightly stronger for it: what those pages carry is 5% to 10% of channels moving by *five levels
  or more*, which is a phase shift on a glyph edge rather than dither.
- **`Tolerance::VECTOR`'s `basic` numbers, re-run.** Today's renderers give reference-against-
  reference means of 0.0016 to 0.0352 and worst tiles of 0.4062 to 1.0625, against "0.002 to
  0.047" and "0.4 to 1.1" written there. The worst tile is exact; the mean's upper end has drifted
  down over four hundred sessions of upstream releases, which is a fact about `poppler` and
  `ghostscript` and not a correction.

## The alternative that was rejected

**Splitting `Tolerance` into a consensus threshold and a judgement floor.** It is the honest shape
of the problem — `Judgement`'s own comment already says the consensus must be decided by the fixed
bounds "since deciding whether the references agree from a bound derived from how much they
disagree would be circular", which is a recognition that the two roles are different — and it is
the only change that would move the 37 pages without moving 278 others. It is rejected *for now*
on the evidence, not on the design: the number that would go in the floor can only be derived
from a pair that includes a non-hinting renderer, and the only one available shares our font
stack. `doc/todo/12` carries it with this ADR's tables attached.

## The cost

A gate that over-reports on text pages, knowingly. 37 pages sit on the contradicted list held to
a bound below the spread of the implementations that set it, every one of them already diagnosed
and ratcheted with an argument that does not depend on the bound. And several hundred pages stay
`ambiguous` that a derived bound would make judgeable — which is `doc/todo/00`'s population, and
which this ADR is the first thing to name a mechanism for.

## What this does not claim

That any page is drawn differently, that any verdict was wrong, or that 0.05 is *right*. The
oracle prints 905 agreeing / 68 contradicted / 786 ambiguous before and after this round, because
nothing that decides a verdict was touched. What changed is that the number now has a derivation
beside it, the derivation disagrees with it, and both facts are written where the next session
reads them.
