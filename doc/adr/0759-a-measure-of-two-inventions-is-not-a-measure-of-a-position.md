# 0759 — A measure built from two inventions is not a measure of a position

Status: accepted.
Context: `crates/pdf-model/tests/text_extraction.rs`'s word-box geometry instrument (ADR 0323's
instrument 1, geometry half; ADR 0421's ratchet; ADR 0726's reading of its tail; ADR 0756's
set-aside), ISO 32000-2 §9.8.1 Table 120, §9.2.2 and §9.4.4, and ADR 0216's plausibility band.

## The defect as it was handed over

ADR 0756's closing section left this, and its reason for leaving it is the whole of the problem:

> The vertical-centre bound divides by the mean of two box heights, one of which this instrument
> has already declared to be each extractor's own convention (ADR 0323's Finding 3). Where the
> reference's box collapses — `issue1350.pdf`'s is 0.146 pt tall and `bug868745.pdf`'s 0.200 pt,
> against ten- and twelve-point words — the denominator halves and the numerator becomes half our
> own height, so the measure reads ≈ 1.0 by construction and says nothing about where the word
> sits. … it was left alone here because every obvious repair lands those two documents within a
> thousandth of the bound, which is a tuned constant wearing a fix.

That last sentence is exact and it is checkable. Dividing by *our* height alone rather than by the
mean puts `bug868745.pdf` at 0.497 against a bound of 0.5 — because this tree's em box is
`(1.0, 0.0)`, one em above the baseline and nothing below it, so its centre is half its height
above the baseline and a reference band hugging the baseline is half our height away. The repair
passes by three thousandths of a quantity nothing derives.

## What the measure is asking, which is the question the round was set

A word box on the cross axis is a **baseline** plus a **band** about it. Writing the band as an
ascent `a` above and a depth `d` below, in a frame whose y grows downward,

    centre = baseline + (d − a) / 2

so for a matched pair

    Δcentre = Δbaseline + ((d_ours − a_ours) − (d_theirs − a_theirs)) / 2.

The first term is what the instrument is named for: §9.4.4's positioning arithmetic on the axis
the text does not advance along, the same quantity `HORIZONTAL_BOUND` is a tolerance on. The
second is the box convention ADR 0323's Finding 3 excluded from the verdict by name. **The measure
is their sum, and it has no way to report either one alone.**

Two consequences follow, and the second is what makes this a defect rather than an untidiness.

**The denominator is not the mistake.** The mean of the two heights is exactly the separation at
which two bands stop overlapping, so the ratio reaches 1.0 precisely when they become disjoint.
That is a well-formed quantity and a good choice for the question *do these two bands overlap*. It
is the numerator that carries the convention, and no denominator repairs a numerator — which is
why every candidate repair to the divisor lands on the witnesses at whatever value the em box's
own geometry dictates.

**The second term is as large as the bound.** Where the file states no band, this tree answers
§9.2.2's nominal line and the reference answers a band of its own, and half the difference of two
such bands is a large fraction of a word's height. The corpus says how large: over the pairs whose
page states no pair, this tree against `pdftotext` reads **median 0.2540, p90 0.3215, p99 0.9813**
of the word's height, against a bound of 0.5. A tolerance is not a measurement when the noise
reaches the tolerance.

## What decides which of the two the measure is reporting, and it is the file

§9.8.1's Table 120 defines both entries **about the baseline**:

> The maximum height above the baseline reached by glyphs in this font.

> The maximum depth below the baseline reached by glyphs in this font.

So a band built from that pair is a statement about where the baseline is, and two readers obeying
the same pair place the same band about it: their centres differ by their baselines and by nothing
else. A band built from anything else — this tree's em box, another reader's font-program metrics
— is a statement about nothing the file said.

`the_band_the_file_states` is that question, asked of the page: `pdf_font::measured_extent` — the
tree's own rule rather than a copy of it, which is what that function is public for — of the
descriptor of every font the page reaches, following a Type 0 font to its descendant's (Table 119
gives the parent none; ADR 0337). A page any of whose fonts states no usable pair keeps **both of
its reading-axis edges in the verdict** and loses the cross-axis measure, counted and printed.

Three details are decisions rather than mechanics:

- **A measure is set aside, not a word.** ADR 0756 took whole words out of the verdict because
  §12.7.4.3 placed them; here §9.4.4's displacement is stated whatever a descriptor says, so the
  reading-axis bound still applies. Neither the judged set nor the pair count moves — 503 and
  11 131 before and after — which is what makes the change legible as a change of measure rather
  than of population.
- **The question is the page's because a test cannot ask it of a word.** `pdf_model::Placed`
  carries a span and a quad and not the font that placed it, so a page one of whose several fonts
  is silent sets aside all of its words' cross axes. That is deliberately the wider condition, on
  ADR 0756's own argument: a set-aside can only take a measurement out of the verdict, never put
  agreement into it, so wide is the safe direction and the printed count is what keeps it honest
  (trap 11).
- **Table 120's own exemption is why a Type 3 page is in the second population.** The pair is
  "Required, except for Type 3 fonts", so `issue6605.pdf` states no band by the standard's
  permission rather than by malformation, and the two mechanisms ADR 0726 filed separately — a
  refused pair and no pair — were one question all along.

## The evidence, and the half of it where neither side is this tree

The gate's own run splits its cross-axis measure into the two populations:

| this tree against `pdftotext` | median | p90 | p99 | max |
|---|---|---|---|---|
| pages stating Table 120's pair (8562 pairs) | 0.0000 | 0.0000 | 0.1041 | 0.6667 |
| pages stating no pair (2569 pairs) | 0.2540 | 0.3215 | 0.9813 | 0.9842 |

Nine of every ten matched words on a page that states the pair are placed by this tree and by
`pdftotext` at the **same** cross-axis centre, to the printed precision. That is what a shared
quantity looks like, and it is the strongest evidence that the first population is measuring
§9.4.4 rather than a convention.

The derivation — `pdftotext -bbox -cropbox` against `mutool draw -F stext`, two extractors sharing
no code, with **this tree on neither side** — was taught the same split and confirms it
independently: the centre bound rejects **0.15% of the 10 073 pairs whose page states the pair and
5.25% of the 2705 whose page does not**, thirty-five times as often exactly where Table 120 is
silent. The condition is therefore corroborated rather than fitted: it was derived from two
printed definitions and then found to separate a population neither side of which is ours.

## What it costs, printed

Four documents leave `SELECTION_BELOW_FLOOR` — `bug868745.pdf`, `issue1350.pdf`, `issue4665.pdf`
and `issue6605.pdf` — and **143 matched words** move from out of bounds to in, which is exactly the
sum of those documents' out-of-bounds words and exactly the verdict's own difference. All four
agreed with `pdftotext` to **0.00 pt on both reading-axis edges**, which is what says the
disagreement was never about where the word is. The verdict moves from 10 951/11 131 words in
bounds to 11 094/11 131, and from 489 of 503 documents fully in bounds to 493 of 503. `JUDGED_FLOOR`
does not move and no refusal count changes.

## What was refused

- **Moving `VERTICAL_CENTRE_BOUND`.** The derivation now shows the stated-band population much the
  tighter of the two, so a smaller bound is available on this evidence. Retightening a bound in the
  same round that changed what it is applied to could not be told apart from fitting it to the
  corpus that came out, and ADR 0323's rule is that a bound moves once its instrument has held
  across rounds. What would justify it is the split holding, quoted off a run.
- **A repair to the denominator.** Every one of them — our height alone, the mean of the two, a
  fixed em — leaves the numerator a sum of a position and a convention, and the two witnesses land
  wherever the em box's own geometry puts them. ADR 0756 refused this and the arithmetic above is
  why it was right to.
- **Replacing the measure with the bands' separation in points**, which is the only cross-axis
  statement that survives both conventions: a band contains its baseline, so two disjoint bands
  prove two different baselines, and the gap is a lower bound on the difference. It was measured
  before it was refused. **The worst pair in the whole corpus reads 0.9842 of a word's height and
  the threshold for disjointness is 1.0** — this tree's band and `pdftotext`'s overlap on every one
  of the 11 131 matched words there are. A verdict that cannot fail anywhere is not a verdict, and
  swapping a bound that fires on a convention for one that fires on nothing would have been the
  worse instrument of the two. **The measure is not vacuous in principle**, which is worth knowing
  before a later round reaches for it again: the derivation's poppler-against-mupdf population does
  reach 4.5295, so two disjoint bands occur between two *references* — and by the argument above
  that pair either differs in baseline or has a band that does not contain one. It is against this
  tree and this reference that the statement has nothing to say.
- **Carrying the font, or the baseline, out of `pdf_model::Placed`** so the question could be asked
  per word. It is a change to a public report type for a test's benefit, and the page-level
  condition is the wider one, which is the safe direction. If a later round wants the baseline in
  the report it should want it for a consumer — a caret has the same need — and not for this.

## What is still owed

- **The cross-axis bound decides no document's verdict today.** After the set-aside, no word in the
  corpus fails it while passing the reading-axis bound; the three that fail it (`issue11555.pdf`)
  fail both. The gate prints that as `0 fail only the vertical centre`. It is a live bound with a
  small population rather than a vacuous one — the stated-band population reaches 0.6667 against a
  bound of 0.5 — but a round that wants it to discriminate should read the derivation's split
  first.

  **What is not left unwatched by any of this is the operational question**, and it is worth naming
  because a narrowed bound looks like a loss until you ask what else asks. `selection_census.rs`
  drags across `pdftotext`'s own box at that box's **vertical midpoint** and requires the selection
  to contain the word — so where a word sits on the cross axis is judged there with the reference
  supplying the y and no convention of ours in the question at all (ADR 0323's instrument 1, second
  half; ADR 0421). It is untouched by this round and prints its own fraction on the same sequence.
- **The em box puts the whole nominal line above the baseline**, `(1.0, 0.0)`, so a selection
  highlight over a font that states no descriptor covers no descender at all. §9.2.2 fixes the
  line's *height* at one unit and says nothing about where the baseline sits inside it, so this is
  a choice ADR 0216 made and did not have to. It is named here because this round's arithmetic is
  what makes it visible — it is why a reference band at the baseline sits exactly at our box's edge
  — and it is not taken here: it moves every selection rectangle in the program and belongs to a
  round that measures them.
- **`issue6127.pdf` is still undiagnosed**, unchanged by any of this, and is still the one where
  both references agree against this tree.

## The floor the calibration asked for

Trap 13 was run in both directions before the change was believed, and only one of them was
caught. Disabling the set-aside fails the gate loudly: the four documents come back and the
ratchet names them. **Widening it to everything failed nothing** — the cross axis is then judged
nowhere, and because the one document whose words fail that bound fails the reading-axis bound as
well, no name enters or leaves the list. A measure can evaporate entirely while the verdict, the
judged set, the refusal table and the named list all stay where they were, which is precisely the
failure this round is about, one level up.

`CROSS_AXIS_FLOOR` closes it: the count of matched pairs the cross-axis bound is applied to is
ratcheted the way `JUDGED_FLOOR` ratchets the judged set, on the same argument and with the same
rule — it may rise, and a fall is written down with its reason. It was added *because* the
calibration came back green in a direction it should not have, which is trap 13 doing the job the
trap is for rather than confirming a sweep somebody already trusted.
