# 0242 — A verdict decided by two renderers and reported against a third

Status: accepted, in the four-hundred-and-sixth session.
Supersedes nothing. Touches `crates/pdf-model/tests/oracle.rs` only — no rendering code, and no
pixel.

## The decision

**A contradicted page's printed line reports the comparison the verdict rests on, and prints every
bound that decided it.** Concretely, in `measurements`:

- On an `Outcome::Regression`, the fold runs over the **consensus** references — the set
  `pdfref`'s `decide` checked us against — and picks the comparison **furthest outside the applied
  bound**, over all four of `Tolerance::accepts`' measures. The printed line is therefore a failing
  comparison, always.
- On an `Outcome::Ambiguous` there is no consensus and no failure to name, so the previous rule is
  kept exactly: the reference we look least like, over all of them.
- The bound half prints **four** numbers where it printed three. `differing` had been printed as our
  measurement with no bound beside it.

And `rank_the_contradicted` prints the ten contradicted pages furthest from their *nearest*
reference — the ranking the ambiguous bucket has had since the hundred-and-seventy-sixth session and
the contradicted list, four hundred sessions older, has never had.

## Why

Two facts, both read off the gate's own output before a line was changed.

**`Tolerance::accepts` applies four bounds and the line printed three.** Not a spare: **thirty of the
sixty-eight contradicted pages printed a line on which every visible number was inside the printed
bound.** A page failing for no stated reason is a page nobody can work, and a group written from
those lines names the wrong measure. `issue7580.pdf` is the plainest — mean 2.93 of 5.00, worst tile
7.10 of 40.00, structural similarity 0.9734 of 0.9000, and `differing 6.15%` against a 5.00% nothing
printed.

**The fold ran over every reference while the bound beside it is the consensus pair's.** So a page
could be reported against a renderer that takes no part in the verdict, held to a bound derived from
the two that do. `smask_luminosity_oob_transfer.pdf` is the witness and it is not marginal: the
consensus is `mupdf` and `ghostscript`, our distance from them is **1.25** against a bound of 1.11,
and the line printed **27.02** — `poppler`, which on that page sits 34 to 36 of 255 from all four of
the other renderers while they sit within 1.7 of each other. `CONTRADICTED_MASK_QUANTISATION` argues
that everybody there is within a level of the arithmetic, and had to state its own numbers because
the gate's were somebody else's.

`measurements`' own doc comment had the principle written down — "the pair that sets the bound is the
*consensus* pair, not the widest" — and applied it to one half of the line.

## What it moved, and it is not a verdict

**Forty-three of the sixty-eight contradicted pages now report a different comparison**, and none of
the 1794 verdicts moved: the oracle is 905 agreeing / 68 contradicted / 786 ambiguous before and
after, and every other gate is unchanged, because no rendering code was touched. Zero of the
sixty-eight lines now pass every printed bound, and **thirty-eight of them fail on exactly one, the
differing fraction.**

The largest thing it moved is a diagnosis. `CONTRADICTED_GLYPH_EDGES` opened, since the
seventy-fifth session, "[e]ach fails **only** on mean absolute difference — 5.4 to 6.4 against a
bound of 5.00". **Every number in that sentence was `ghostscript`'s, and `ghostscript` is in the
consensus on none of the group's 21 pages** — all of them read "poppler and mupdf agree". Against the
pair that decides them the means are 1.01 to 2.57 of 5.00, the worst tiles 2.67 to 9.53 of 40.00, the
structural similarities 0.9655 to 0.9981 of 0.9000, and **all 21 fail on the differing fraction and
on nothing else**, at 1.00 to 1.56 times each page's own bound.

**The group's diagnosis survives and gets sharper, which is why the correction is worth more than the
error.** `differing_fraction` counts channels that moved at all; `mean_error` weighs how far. A glyph
drawn at a different sub-pixel phase moves every pixel of every outline a little — a large count and
a small average. A population failing the count and nothing else *is* "the ink is right and its
placement inside the pixel is not", stated arithmetically. The ink table in that group was already
saying it; so was the bound, and nobody could read it.

## The alternative that was rejected

Folding `Distance::of` into the new four-measure ratio. It is the same idea and it would have made
one function where there are two — but `Distance`'s numbers are quoted in something like a hundred
entries of `oracle.rs` and throughout `doc/todo/00`, and a page recorded as "0.16 from the nearest
reference" has to stay the number that was recorded. The two ratios are kept apart and the reason is
written at `outside_by`.

## The cost

Two ratio functions instead of one, and a `measurements` with two rules in it. Both are documented
where they are, and the second rule is not arbitrary: on an ambiguous page there is no consensus, so
"furthest outside the bound" is not defined and "the reference we look least like" is the honest
summary. The line is also four characters longer per page across 854 printed pages, which is
nothing.

## What this does not claim

That any page is drawn differently, or that any page's verdict was wrong. Every verdict was reached
by `pdfref`'s `decide` on the consensus comparisons all along; what was wrong is what the gate *said*
about it. The two rankings the round now has disagree about which page heads the contradicted list —
in bounds it is the JBIG2 pages, in levels of 255 it is `bug847420.pdf` — and neither is a defect in
the other. `rank_the_contradicted`'s comment says which question each asks.
