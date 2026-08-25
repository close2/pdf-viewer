# 0663 — The numerator a ranking never required, and the four pages it pointed at

**Status.** Accepted. Session 751.

Takes both of the questions ADR 0647 left owed and answers them together, because they are one
question: what the *we are alone* list should contain, and what the list as it stood was pointing
at. **No pixel moves and no verdict moves** — the whole diff under `crates/` is `tests/oracle.rs`,
a test target no library links, and every one of the 969 non-agreeing per-page lines and every
census figure is identical between the run before this round and the run after it.

## Context

`doc/todo/00` step 1 asks which pages *we are alone on*: our distance from the **nearest** reference
larger than the distance between the **closest two** references, both in `Distance`'s three
measures. ADR 0647 gave the gate that ranking and then read it, and the reading was mostly about the
ratio rather than about a page:

> Neither column has a floor at 1. On **31 of the 48** the closest pair sits inside all three bounds
> — the page is ambiguous on the differing fraction alone — and on **22** our own nearest is inside
> them too, so the ratio is between two numbers that agree with everybody and it ranks a page higher
> the more closely the references agree.

The head was the sharpest instance: `issue11403_reduced.pdf` at 9.06×, ours 0.51 over 0.06, on a
page whose verdict line reads `differing alone` — a disagreement invisible to the three measures the
ratio is computed in. 0647 declined to act on that and handed on the question, and it named four
pages of its own sublist it had not opened: `bug766086.pdf`, `issue16224.pdf`, `endchar.pdf` and
`issue12337.pdf`.

It also added trap 9's tenth mechanism, which is the reason both halves are one round: **every way
shared code manufactures an agreement acts on this ratio's divisor, where it accuses us instead of
excusing somebody.** 0647 measured that on one book. Whether the four pages are the same shape was
the open question about the list's contents.

## The decision, part one: the numerator has to be outside a bound

`rank_the_pages_we_are_alone_on` requires our own nearest to sit **outside at least one of the three
bounds**. The pages that drops are counted underneath rather than left to a caution.

Three things make 1.0 the honest place to cut, and the first is the one that matters.

**On an ambiguous page the bound is the tolerance class's own floor, not a judgement anybody made.**
`pdfref::decide` returns the class `Tolerance` *unwidened* where no consensus formed — widening is a
consensus's, derived from its own members' spread — so `ours > 1` means outside the fixed floor for
this page's class, the same constant for every text page in the pool. That answers the objection
`Examined::outside_the_bound` records, which is why this round could not simply assert the cut:
that doc comment declines to *rank* ambiguous pages against their bound because "the bound beside
them decided nothing". It is right, and it is about ranking by a page-dependent quantity. Testing a
page-**independent** threshold is a different operation, and the class floor is page-independent by
construction.

**Below 1 the numerator says the opposite of the list's name.** Our nearest inside all three bounds
means that reference, had it been in a consensus, would have *accepted* this page. A page somebody
accepts is not a page we are alone on, on this instrument's own terms.

**And the cut was measured before it was taken.** The list goes from 48 pages to 26. The printed ten
lose exactly one — `issue11403_reduced.pdf`, the page 0647 named as the defect — and gain exactly
one, `endchar.pdf`, which was rank eleven and is in the sublist. Nothing else moves, and **no page
that the ranking never printed becomes the head**, which is ADR 0349's standing warning and the one
failure a filter on a ranking can produce.

Two consequences are written into the printed line:

- **The count underneath is the sublist now, not the population.** With the numerator required
  outside, the pages whose *denominator* is inside all three bounds are exactly step 1's queue: we
  fail the floor against every reference, and the closest two references pass it against each other.
  That is **9 of the 26**, which is 0647's sublist reproduced by the instrument that now defines it.
- **The pages dropped stay printed as a number**, because a caution nobody can count is trap 11 with
  the sign reversed — the same rule the function already applied to the two counts it replaced.

**In four measures the same requirement changes the count by nothing**, and that is the asymmetry
stated as arithmetic rather than as a caveat: `consensus_missed_by` is above 1 on every ambiguous
page by construction, because a pair inside all four bounds would have been a consensus, so *ours >
theirs* already implied *ours > 1* there. The three-measure denominator has no such floor. The
filter is written into that count anyway, so that a reader comparing two counts of one shape can see
the same question was asked, and so that a fifth measure joining `Tolerance` has to re-check the
implication rather than inherit it.

## The decision, part two: the four are four different shapes

Opened against the ranking rather than against their own groups' arguments. Each page's numerator is
decomposed to the measure that drives it and each denominator to the pair that forms it:

| page | our number is | the divisor is | and that is |
|---|---|---|---|
| `bug766086.pdf` | 2.58, the **similarity**, against `poppler` | `mupdf` + `ghostscript` at 0.45 | trap 9's *shared gap* |
| `issue16224.pdf` | 1.13 against `mupdf` | `poppler` + `mupdf` at 0.41 | trap 9's **tenth** mechanism |
| `endchar.pdf` | 1.97, the **mean**, against `mupdf` | `poppler` + `ghostscript` at 0.83 | neither |
| `issue12337.pdf` | 1.12, the **mean**, against `ghostscript` | `mupdf` + `ghostscript` at 0.88 | neither |

**So a sublist is not a diagnosis.** One of the four is the mechanism 0647 measured on the book, one
is a different bullet of the same trap, and two are not that trap at all. The shape *we are outside
and they are inside* is worth opening precisely because what is behind it differs page by page.

### `bug766086.pdf` — the ratio measures one annotation twice

`AMBIGUOUS_LINK_BORDER` said "[o]urs and `poppler`'s draw it" and priced the ink: ours 20.61 against
`poppler`'s 20.73. The ink is right and it hid a pixel, which is trap 12's second paragraph one
group over — the measure this page's 2.58 is taken on is the **structural similarity**, not the
mean. At 72 dpi on a 200 × 50 page a user unit is a device pixel, `/Rect [5 10 190 40]` is device
columns 5 to 190 and rows 10 to 40, and a one-unit border completely inside it occupies columns 5
and 189 and rows 10 and 39. Read off the two rasters:

```text
          left  right      top  bottom
ours         5    189       10      39
poppler      5    190       10      40
```

§12.5.4: "If present, the border shall be drawn completely inside the annotation rectangle." Ours is
that sentence; `poppler`'s column 190 covers x ∈ [190, 191] and its row 40 covers user-space
y ∈ [9, 10], both outside the rectangle. It is `AMBIGUOUS_OVERSIZED_BORDER`'s finding at a width of
1 instead of 112 — the same renderer centring the stroke on the rectangle's edge — and at that width
the ink cannot see it while the similarity can.

**And both halves of the page's 5.68× are that annotation**, which was measured by removing it:
`/Annots [4 0 R]` replaced by `/Annots []` in place, same byte length so the cross-reference table
still resolves, all four renderers re-run.

```text
                       with the annotation         without it
ours vs poppler        mean 7.5845 ssim 0.74205    mean 2.1275 ssim 0.98237
ours vs mupdf          mean 6.7478 ssim 0.60519    mean 1.3163 ssim 0.99105
mupdf vs ghostscript   mean 2.2695 ssim 0.98268    mean 2.2695 ssim 0.98268
```

Our number falls from 2.58 bounds to **0.43** — inside every one of them — while the pair the ratio
divides by is **byte-identical to the digit**, because neither of those two draws the annotation at
all. `mupdf` constructs no `Link` appearance; `ghostscript` is being asked to print and Table 167's
Print flag is clear. So the numerator is a clause we implement and the divisor is the same clause
two renderers do not.

### `issue12337.pdf` — the numerator is the finding

One `/Highlight` with **no `/AP`**, `/Rect [48.75 300 297 443.25]` and a single `/QuadPoints`
quadrilateral identical to it. §12.5.6.10 states the region and nothing about the marks in it, which
is `AMBIGUOUS_MARKUP_ARTWORK`'s subject; the standard states *this* region twice, in Table 166's
`/Rect` and in that `/QuadPoints`. What it says about marks outside it is nothing at all — the
nearest sentence is §12.5.5's, of an appearance a file supplies rather than one a processor builds:
"Each appearance stream is a form XObject (see 8.10, "Form XObjects"): a self-contained content
stream that shall be rendered inside the annotation rectangle." Yellow pixels, per renderer:

```text
ours          x  49 .. 296     inside, flush with both edges
poppler       x  22 .. 323     27 columns left of it, 26 right
mupdf         x  23 .. 321     26 left, 24 right
ghostscript   x  31 .. 314     18 left, 17 right
hayro         no yellow at all
```

All four that draw it agree about the rows and bulge only sideways, and the page's worst tile, at
(288, 416), is exactly where our yellow stops and theirs continues. Removing the annotation the same
way moves our nearest **1.12 → 0.61** and the divisor 0.88 → 0.89, so without it the page is not on
the list at all. The ranking is right that we are alone here, and the reason is that we are the only
one of five inside the stated region — a documented choice on the side the one relevant `shall`
points, rather than a `shall` this file's annotation is literally under.

`AMBIGUOUS_ONE_LADDER` holds this page on an *ink* argument, and an ink number is a page quantity
where this disagreement is a place. The ladder is not wrong; it could not see this.

### `issue16224.pdf` — trap 9's tenth mechanism outright

One line of an embedded Type 1C subset of `MyriadPro-Regular` on a 183 × 33 page:

```text
poppler vs mupdf        mean  2.0394   ssim 0.98174     0.41 bounds
poppler vs ghostscript  mean 10.0894   ssim 0.68934     3.11 bounds
mupdf   vs ghostscript  mean  9.9475   ssim 0.69494     3.05 bounds
ours    vs mupdf        mean  5.3409   ssim 0.88671     1.13 bounds
```

The two that share `libfreetype.so.6` are seven and a half times closer to each other than either is
to the `ghostscript` that links its own copy, and **we are less than half as far from `mupdf` as
`ghostscript` is**. The page rises to 2.78× because the divisor is one glyph rasteriser.

**And 0647's book-sized measurement is reproduced over the whole pool.** Taking every ambiguous
page's closest pair by name: `poppler` + `mupdf` is the closest pair on **23 of the 48** and on
**137 of the other 788**, while `poppler` + `ghostscript` — the one pair of the three sharing no
glyph rasteriser — is closest on **2 of the 48** against **333 of the 788**, where it is the
commonest closest pair there is. Enriched almost threefold in the sharing pair, depleted tenfold in
the pair that does not share.

### `endchar.pdf` — a one-ladder limit given three more

The group's limit for this page stood on `poppler` alone, which `doc/todo/02` §7 says is not a
limit. Ink as `(1 − mean) × 255` after `-alpha off -channel R -colorspace Gray`, each renderer on
its own uncropped raster:

```text
                  72       288       576      2304
ours          59.4874   59.8367   61.1486   60.9729
mupdf         58.1554   59.9419   60.6850   60.9314
poppler       59.0589   60.3054   60.8458   60.9757
ghostscript   59.6630   61.4630   60.9818   61.0843
```

Four independent ladders land within **0.153 of 255** of each other, ours between `mupdf`'s and
`poppler`'s. The coverage is agreed and what is left at the page's own scale is a spread of 1.51 of
255 over a raster **15 × 34 device pixels** — §10.7.4's "[s]can conversion of character glyphs may
be performed by a different algorithm from the preceding one", which is this group's clause.

**Three of the four 72-dpi figures the note was written on reproduce and the fourth is ours**:
`poppler` 59.0589 against a recorded 59.06, `ghostscript` 59.6630 against 59.66, `mupdf` 58.1554
against 58.16 — and ours **59.4874 against a recorded 59.39**. The number that moved in the rounds
between is this tree's own, on a fifteen-column raster, while every reference is where it was.

## Consequences

- **The instrument to copy is the removal.** A ratio's two halves can be the same mechanism, and no
  number on the printed line can say so. Editing the document so the mechanism cannot act and
  re-measuring *both* sides is trap 9's own instrument turned on our own accusation instead of
  somebody else's excuse, and it settled two of these four pages in minutes.
- **The list is smaller and says more.** 26 pages instead of 48, a head that differs by one page,
  and a count underneath that names the queue instead of the population it was drawn from.
- **Nothing is hidden.** Every page dropped is still in the ambiguous bucket, still held by name in
  an `AMBIGUOUS_*` group, still swept by `doc/todo/00` step 7 — which is the instrument for content
  a distance cannot see and is orthogonal to this one — and still counted on the line under the
  list.
- **What is still owed.** Nine of the twenty-two dropped pages are `standard_fonts.pdf` and four are
  `freeculture.pdf`; nobody has asked whether that concentration is the same denominator effect at
  work on a population where our own number is inside. And `doc/todo/12`'s 278 pages are unchanged
  from 741 and 744.
