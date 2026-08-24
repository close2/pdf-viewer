# ADR 0616 — The consensus that was not unique, and the four verdicts an enumeration order decided

Status: accepted, 2026-08-25. Session 727. Makes `pdfref` count and report **every** maximal
agreeing set of references rather than one; adds `DIVIDED_CONSENSUS` and a census line to
`oracle.rs`; rewrites three `CONTRADICTED_*` notes; amends trap 12.
**No pixel moves, no verdict moves and no list loses a page** — every one of the 1945 per-page
verdict lines is byte-identical to the run before the change.

## The criterion, and why it is the sixth one and not a seventh

722 pointed ADR 0497's sixth criterion at the whole contradicted pool and answered it: every
failing bound in the pool is now named by the note that holds its page. Its own account of what
that leaves is the instruction this round took:

> `unpriced` cannot tell a bound *named* from a bound *accounted for*. The vocabulary is complete
> now; whether each mechanism owns its margin is still the sixth criterion done by hand.

The sixth criterion has a **control clause** as well as a question, and ADR 0606 exercised it on
exactly two pages: *taking us out of the room does not rescue the bound, because two other
renderers fail it too.* That control is a statement about the **consensus** rather than about our
render, and it is the one half of the criterion nobody had asked of the pool. So the criterion
this round states is the sixth's control, pointed at the population:

> For a page held contradicted, is the consensus the verdict rests on the *only* reading the
> references had of that page?

It needs no rasteriser and no new idea. The gate already computes every pair.

## What it found

**Agreement is not transitive, and nothing in this tree had said so out loud.** `pdfref::decide`
takes the largest set of references that all agree with one another, and its own comment is right
about why that is not "count pairwise agreements": *A agreeing with B and B with C does not make A
agree with C.* What the comment does not follow through is the consequence. With three references,
`a ~ b` and `b ~ c` while `a ≁ c` leaves **two** maximal agreeing sets, `{a, b}` and `{b, c}` —
neither contained in the other, and neither a majority in any sense the other is not.

The loop skipped `subset.len() <= best.len()`, so the second was discarded without being counted.
Which one survives is decided by the subset bitmask, which is the order `Reference`'s variants
happen to be declared in.

Measured over the corpus the gate walks, the population is small and the consequential part of it
is smaller: the run prints how many pages carry more than one maximal consensus, and how many of
those the sets **disagree about us** on. The second number is four, all four contradicted, and all
four would have *agreed* under the set that was thrown away:

| page | the set the gate took | the set it discarded | what the discarded one says |
|---|---|---|---|
| `colorkeymask.pdf` p1 | `poppler` + `mupdf` | `ghostscript` + `mupdf` | agrees |
| `colors.pdf` p1 | `poppler` + `ghostscript` | `ghostscript` + `mupdf` | agrees |
| `colors.pdf` p2 | `poppler` + `ghostscript` | `ghostscript` + `mupdf` | agrees |
| `issue11403_reduced.pdf` p1 | `poppler` + `mupdf` | `poppler` + `ghostscript` | agrees |

Four of the sixty-five contradicted pages are contradicted on the strength of an enumeration
order. Each was reproduced by hand from the run's own artefacts through
`examples/compare_rasters`, which is the same arithmetic the gate uses pointed at two files.

### `colorkeymask.pdf` page 1 — the consensus that was available contains a renderer identical to us

`CONTRADICTED_IMAGE_SAMPLE_AT_THE_PIXEL_CENTRE`'s note has said since the four-hundred-and-forty-third
session that ours and `ghostscript` are **byte-identical over the whole 595 × 842 raster** and that
`poppler` "votes with `mupdf`". Both halves are true and the second is incomplete: `ghostscript` and
`mupdf` agree here as well, inside every class bound, and `poppler` and `ghostscript` are the pair
that parts. So a consensus existed containing a renderer our raster cannot be distinguished from,
and a consensus containing such a renderer cannot contradict us. The page fails on the worst tile,
5.03 against 5.00.

This one is the cleanest statement of the defect because it needs no tolerance arithmetic at all:
the rival consensus accepts us by identity.

### `colors.pdf` pages 1 and 2 — and the pair that decides them is not the tightest on the page

This is the group named `CONTRADICTED_TIGHT_CONSENSUS`, and 722 had already measured every pair on
both pages while reading four of the ten rows. The sixth row is the one nothing had asked about:

```text
                             page 1     page 2
  ghostscript <-> mupdf      0.99625    0.99278    ssim — and mean, and worst tile on p1
  poppler <-> ghostscript    0.99431    0.99201    the pair the verdict rests on
  ours                       0.98786    0.98024
```

`ghostscript` and `mupdf` agree with each other **more closely** than the pair that votes, on three
of the four measures on each page — mean and worst tile as well on page 1, mean and the differing
fraction on page 2 — and they agree inside every class bound, so `{ghostscript, mupdf}` is a
consensus of exactly the same standing. Under it the widened structural bound is *tighter* than the
class floor, so the floor of 0.9900 applies, and our worst structural similarity against that pair
is `ghostscript`'s 0.99627 on page 1 and 0.99336 on page 2 — inside, with the other three measures
inside as well.

**That does not withdraw a line of ADR 0474's or ADR 0606's measurements**, which are taken against
the page's own closed form with no renderer in them. What it does is put a second half on trap 12: a
bound derived from two agreeing references can be tighter than the arithmetic, *and* the two
agreeing references need not be the two that agree most.

### `issue11403_reduced.pdf` page 1 — rescued by the looser pair, which is the other direction

`CONTRADICTED_SUBSTITUTED_FONT`'s note already records that the pair the gate calls agreement here
"differs by a mark one of them invented" — `mupdf` draws a stray acute accent 32 device columns to
the left of the line. The rival is `poppler` and `ghostscript`, which agree inside every class bound
at 4.815% of channels against the 5.00% the class allows, while `ghostscript` and `mupdf` part at
5.16%. Twice 4.815% is a differing bound of 9.63% and our 6.24% is inside it.

So here the page is rescued by the pair that agrees **least** — the opposite of `colors.pdf`. Both
directions occur in a population of four, which is the whole reason the next section exists.

## Decision — count it, name it, and move nothing

The four verdicts rest on an enumeration order. What should replace that is a separate decision,
and this round does not take it. Three rules are order-independent and each has a hazard:

| rule | today's cost | the hazard |
|---|---|---|
| contradicted iff **every** maximal consensus rejects us | none — no page currently agreeing has a rejecting rival | it is the strictest reading, so it keeps four verdicts nobody can defend on the merits |
| **`ambiguous`** where the sets disagree | four diagnosed pages leave the pool for the least-watched bucket | a page can escape contradiction because a *worse*-agreeing pair exists and its doubled spread is wide, which is `an_outlier_reference_does_not_widen_the_bounds` arriving by another road — `issue11403_reduced.pdf` is exactly that shape |
| take the **tightest** set | three pages move | "tightest" needs an order over four measures, and on `colors.pdf` the two candidate pairs each win two of them on one page and three on the other |

ADR 0499 set the precedent for this gate and it is the right one: a change that moves pages between
lists "would be the honest instrument … [i]t would also move pages between four lists at once, and
trap 11's rule — a report is only as good as the condition it fires on — makes that a decision with
its own ADR rather than a corollary of this one." Session 651's rule is the same sentence about a
tally. **A verdict moves on an argument, not on a discovery**, and the discovery is a day old.

What was taken instead:

- `pdfref::Consensus` and `Triangulation::consensuses` — every maximal agreeing set, largest first,
  each with what it concludes and the bounds it holds us to. `decide` takes the first, which is
  what it has always taken; the change is that the others exist rather than being skipped.
- The gate counts pages carrying more than one, and **names** every page where they disagree —
  `doc/todo/02` §6's rule that a count beside a list is not the list.
- `DIVIDED_CONSENSUS` holds the four with the reading of each, and the census names any page in the
  population that is on no list, which is `JUDGED_WITHOUT_A_THIRD_READING`'s shape one bucket over.
- The three group notes holding those pages say so, each citing this ADR, which is also what keeps
  them off `--bin overtaken`.

## Calibration, per trap 13

The instrument was written against a defect before it was pointed at the corpus.
`two_maximal_consensuses_can_disagree_about_us` builds three panels a step apart so that the outer
two are two steps from the middle one and four from each other, with a tolerance a fifth wider than
one step: `{poppler, mupdf}` and `{mupdf, ghostscript}` are both maximal, and against a render one
step from the middle they reach **opposite** verdicts. It asserts both, and asserts that the taken
outcome is still the first — the property that makes this change verdict-neutral.
`a_unanimous_agreement_is_one_consensus_and_not_four` pins the other side: where all three agree,
the pairs inside that set are not separate consensuses.

## Consequences

- Trap 12 gains the second half: **a tight consensus need not be the tightest, and the tie is
  broken by a declaration order.** The tell is that the gate names a pair while a third pair sits
  inside the same bounds.
- Trap 9 is untouched and is worth distinguishing from this. Its nine mechanisms are all reasons an
  agreement is not evidence; this is a case where there are *two* agreements and the instrument
  reported one. `colorkeymask.pdf` is the sharpest illustration of the difference: nothing is wrong
  with `poppler` and `mupdf` agreeing there, and the verdict is still not the page's.
- `--bin unpriced` reads a fourteenth note and its pool of named bounds grows with it; every failing
  bound in the pool stays named, which is the property 722 established.
- Owed, and this ADR's own residue: **the rule.** Whoever takes it should measure the second and
  third columns of the table above over the corpus rather than over four pages — the census makes
  that a run rather than a project — and should say what happens to a page whose rival consensus
  arrives *after* a pixel moves, which is the direction none of the four is in today.
