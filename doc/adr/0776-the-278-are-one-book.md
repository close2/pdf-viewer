# 0776 — The 278 are one book, and formation is the floor by another route

Status: accepted, 2026-09-01. Session 849, an oracle round on `doc/todo/12` item 1 — the
consensus half, handed on by ADR 0774.

ADR 0243 measured once that raising `Tolerance::max_differing_fraction` for **consensus
formation** makes several hundred `ambiguous` pages judgeable and leaves 278 of them
contradicted, and declined to move the number because "278 pages nobody has looked at" is a
programme of work rather than an argument. Nothing since had said what those pages **are**.

The gate counts the counterfactual now, on every run, off comparisons it already had. **The
threshold does not move, ADR 0243 is extended with the composition, and `doc/todo/12` has
nothing left.** Two findings decide it, and neither was available to ADR 0243:

- **The population is one book.** 272 of the 276 pages the raise newly convicts are
  `freeculture.pdf`; the other four are one page each. On 263 of the 276 a reference agrees
  with **us** more closely, on the deciding measure, than the convicting set agrees with itself.
- **The formation half is the floor half by another route.** Raising formation *alone* acquits
  27 of the 60 contradicted pages, and among them are all six pages ADR 0771 refused the floor
  raise for. `Tolerance::widened_to` is the route: a set admitted at a wider spread widens the
  bound derived from it.

## 1. The instrument

`pdfref::Triangulation::rejudged(formation, floor, judgement)`, and `decide` split into the two
tolerances it always had one of. The live gate passes the same value for both — which is
`doc/todo/12`'s title stated in the code — and the counterfactual passes a raised formation
bound with the floor taken two ways.

It runs **`decide` itself**, not a second implementation of the consensus rule, over
`Triangulation::between_references` and `Triangulation::ours`, which every page already holds.
No render, no comparison, no verdict: the gate reports the same 980 / 60 / 836 as before.

**Its calibration is an assertion over the whole corpus, every run.** Handed the page's own
bounds, `rejudged` must reproduce the verdict the page actually got; `RaisedFormation::control`
carries that and `a_raised_formation_bound` fails naming any page where it does not. A
counterfactual that cannot reproduce the fact is measuring its own arithmetic — trap 13, one
directory over — and two `pdfref` unit tests pin both halves of that: the identity on the three
shapes a verdict has, and that a wider formation bound can in fact form a consensus that then
convicts, so the identity is not vacuous.

**The raise is ADR 0243's own rule and today's number.** The 99th percentile of the
reference-against-reference differing fraction, re-derived this session by
`the_fixed_bounds_against_the_references_own_spread`: on text pages **11.21%** within the pair
sharing one `libfreetype.so.6` and **12.04%** across the boundary, against a bound of 5.00%; on
vector pages **1.36%** and **1.11%** against 1.00%. So 0.12 for text — the number ADR 0243 ran —
and 0.0136 for vector, which ADR 0243 did not raise and which is here so that *is this a fact
about text pages or about the measure?* is answered rather than assumed.

## 2. The two arms

| | our floor left at the class bound | the floor raised with it (ADR 0243's arm) |
|---|---|---|
| `agrees` → `contradicted` | 1 | 1 |
| `contradicted` → `agrees` | **27** | 36 |
| `contradicted` → `ambiguous` | 1 | 0 |
| `ambiguous` → `agrees` | 217 | 217 |
| `ambiguous` → `contradicted` | **276** | 276 |
| unchanged | 1354 | 1346 |

Over the 1876 pages a verdict is reached on. ADR 0243's arm reproduces its own finding four
hundred sessions later — 493 pages leave `ambiguous` where it counted 457, 276 contradicted
where it counted 278 — which is a corpus and three renderer releases, not a correction.

## 3. What the 276 are made of

| | |
|---|---|
| by class | **275 text**, 1 vector |
| by document | **272 `freeculture.pdf`**, 1 `bug766086.pdf`, 1 `issue12337.pdf`, 1 `issue16224.pdf`, 1 `transparency_group.pdf` |
| by the set that would convict | 263 `poppler` + `ghostscript`, 7 `mupdf` + `ghostscript`, 5 all three, 1 `poppler` + `mupdf` |
| by the measure that convicts | **274 structural similarity**, 2 worst tile, **0 the differing fraction** |
| by how much of the raise they need | 205 past 10%, 62 past 8%, 8 past 6%, 1 within a fifth |
| a set whose members drew the same bytes | **0** |
| a reference nearer to *us* than the set is to itself, on the deciding measure | **263** |

Five things follow, and each is a row of that table rather than a reading of it.

**It is the text class's question, as ADR 0243 said.** The same derivation applied to the vector
class moves one page in the whole corpus.

**It is one document.** "278 pages nobody has looked at" is, on today's corpus, one 1917
dense-text book and four single pages. That is `doc/todo/00`'s dense-text population, which
`doc/todo/12` item 1 guessed at — "several hundred are `doc/todo/00`'s dense-text population" —
and which is now counted instead of guessed. A programme of 278 diagnoses and a programme of
one document plus four pages are different objects.

**The convictions rest on a measure the raise does not touch.** The raise's entire justification
is that the differing fraction sits below its own references' spread. Not one of the 276
convictions is *on* the differing fraction: 274 are on structural similarity, whose bound
`Tolerance::TEXT_HEAVY` chose deliberately — "0.90 is a deliberate choice about *which*
population to exclude rather than a natural boundary, and what it excludes is font
substitution". Raising one bound to admit a pair, and then convicting on the bound that was set
to make such pages `ambiguous`, is not the change the derivation argues for.

**The raise cannot be made smaller.** 205 of the 276 form only past 10% differing, so a raise to
8% would buy a seventh of the population at the same price in acquittals.

**And on 263 of the 276 the convicting set is not the closest reading of the page.** Our own
render is nearer to some reference, on the very measure the conviction rests on, than the two
convicting programs are to each other. That is trap 12's subject — `decide` takes the *closest*
pair in the room, and a wider formation bound is precisely an instruction to accept a less close
one — and it is a like-for-like comparison, one measure against the same measure (ADR 0688).

## 4. The four pages outside the book, read one at a time

Every pair on each page, `examples/compare_rasters` over the gate's own panels:

**`bug766086.pdf` page 1** — the convicting set is `mupdf` + `ghostscript` at ssim **0.98268**,
and neither of those two draws the page's link border: `mupdf` constructs no `Link` appearance,
`ghostscript` renders for paper. ADR 0663 priced that removal — take the annotation out and their
comparison is byte-identical to the digit. So the conviction the raise buys here rests on trap
9's first mechanism, a **shared gap**, at 4.10× the bound. It is also the one page of the 276
where nobody is nearer to us than the pair is to itself, because two renderers omitting the same
thing agree exactly.

**`issue16224.pdf` page 1** — the convicting set is `poppler` + `mupdf` at ssim **0.98174**, the
one voting pair that hints through a single `libfreetype.so.6`, on a page whose content is one
line of an embedded Type 1C subset. ADR 0663 named this page for that reason; the raise turns it
into a conviction at 1.24×.

**`issue12337.pdf` page 1** — the set is `mupdf` + `ghostscript` at ssim 0.94038 while **ours
against `mupdf` is 0.96606**, closer than the convicting pair is to itself. Convicted on the
worst tile at 1.16×.

**`transparency_group.pdf` page 1** — the one vector page, and the sharpest of the four. The set
is `mupdf` + `ghostscript`, admitted at **1.0076%** differing by a raise from 1.00% to 1.36%,
while our own render sits at mean **0.1600** and differing **0.3870%** from `poppler` — a quarter
of the distance any two references are from each other. Convicted on the worst tile at 3.19×.

## 5. The book

`freeculture.pdf` is 8-point body text at 72 dpi, where every glyph edge is a differing channel.
Pages 100 and 13, all six pairings, on the gate's own panels reconciled to one size:

| page 100 | mean | differing | ssim |
|---|---|---|---|
| `poppler` v `mupdf` | 5.39 | 10.60% | 0.8406 |
| `poppler` v `ghostscript` | 4.15 | 11.19% | **0.9315** |
| `mupdf` v `ghostscript` | 6.88 | 13.29% | 0.8052 |
| **ours v `mupdf`** | **3.13** | 11.63% | **0.9558** |
| ours v `poppler` | 5.75 | 13.14% | 0.8538 |
| ours v `ghostscript` | 7.05 | 13.04% | 0.8124 |

Page 13 is the same table to two decimals. **The tightest agreement on the page is ours with
`mupdf`**, on both the mean and the structural similarity, and the pair that would convict us is
neither the closest pair nor a pair that agrees within the class bound on anything but ssim. What
the raise does here is admit the one pair whose ssim happens to clear 0.90, then hold us to a
bound widened from *their* ssim and not from the 11.19% of channels they differ on.

**And the pages were looked at, which is trap 1.** At 4× over the same three lines, ours,
`poppler` and `mupdf` are one picture — same shapes, same positions, same weight — and
`ghostscript` is visibly lighter with its own sub-pixel placement, colliding the quotation marks
in *The "copy-right" was*. The page is not in dispute; its glyph edges are.

## 6. Why the threshold does not move

Six reasons, in the order they were found, and none of them is "the count would go up":

1. The population is one document and four pages, so the raise buys almost no *breadth* of
   judgement — it converts one book from `ambiguous` to contradicted.
2. Not one conviction is on the measure the derivation is about.
3. On 263 of 276 the convicting set is not the closest reading of the page; ours with a
   reference is.
4. Two of the four pages outside the book are convicted by mechanisms trap 9 already names —
   a shared gap and a shared glyph rasteriser — and both were already on record (ADR 0663) as
   *denominator* problems rather than defects of ours.
5. A smaller raise buys a seventh of the population at the same price.
6. And the price is the one ADR 0771 already refused, which is §7.

## 7. The finding that closes the item: the two jobs are not separable in either direction

ADR 0243 named the narrow move — *keep 5% for consensus, floor our own judgement higher* — and
ADR 0771 measured it and refused it, because six pages would be acquitted whose mechanisms are a
§8.6.5.3 colour reading and a §10.7.4 departure: a differing fraction is a threshold count, so a
bound cannot separate what a mechanism separates.

Item 1 is that move's mirror: raise formation, leave our floor alone. **It is the same change.**
With the floor held at the class bound, the raise still acquits **27 of the 60** contradicted
pages, and among them are `calrgb.pdf` pages 1, 5, 11 and 12, `issue9940.pdf` page 1 — all five of
`CONTRADICTED_CALRGB_TO_SCREEN` — and `issue4436r.pdf` page 1, which is
`CONTRADICTED_SUBPIXEL_IMAGE`. Every one of the six ADR 0771 refused the floor raise for.

The route is `Tolerance::widened_to`, and it is one sentence: **a bound is derived from the
spread of the set that formed, so admitting a wider set widens the bound.** On `calrgb.pdf` page
1 the raise lets `poppler` join `mupdf` and `ghostscript`, whose widest internal spread is then
11.65% differing, and twice that is what we are judged by. The formation bound is not a knob that
decides only *who votes*; through the widening it decides *what we are held to* as well.

So `doc/todo/12`'s question — can one number be two? — has the same answer from both ends, and
the answer is now measured from both.

## 8. What this changes

- `pdfref`: `decide` takes a formation bound and a floor; `Triangulation::rejudged` is the
  counterfactual; two unit tests, one of which is the calibration and one of which is its
  non-vacuity.
- `oracle.rs`: `RaisedFormation`, `Standing`, `a_raised_formation_bound`,
  `what_the_new_convictions_are_made_of` and `the_pages_a_raised_formation_bound_would_move` —
  the matrix, the composition, and the two small populations named outright.
  `PDFVIEWER_ORACLE_FORMATION=1` prints all 276 rather than the head.
- `Tolerance::TEXT_HEAVY`'s comment carries the composition beside the count it already carried.
- Trap 12 gains the widening route, which is what makes item 1 and ADR 0771's move one change.
- `doc/todo/12` item 1 is answered; nothing in that file is owed, and the freeculture population
  is `doc/todo/00`'s where it always was.
- No bound, verdict, page or pixel moves: **980 agrees / 60 contradicted / 836 ambiguous** over
  1945 pages before and after.

## 9. What this does not claim

That the 272 `freeculture.pdf` pages are right. They are `ambiguous`, which is the verdict for a
page whose references cannot agree, and §5's table says why they cannot: three renderers scatter
over a dense-text page at 72 dpi by more than any of them sits from us. What is denied is only
that a raised formation bound turns that scatter into evidence — on 263 of the 276 it convicts on
a measure the raise does not touch, by a set that is not the page's closest reading.

Nor that `poppler` + `ghostscript` is a manufactured consensus in trap 9's sense: it is the one
voting pair that shares neither `libfreetype.so.6` nor an Artifex source tree, and its appearing
263 times here is a fact about which pair clears an ssim threshold on dense text, not about
shared code.
