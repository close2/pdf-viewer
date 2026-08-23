# ADR 0499 — The eight notes, and the numbers five of them owed

Status: accepted, 2026-08-23. Session 675. Points ADR 0497's sixth criterion at the population it
named: six of the eight middle-bucket `CONTRADICTED_*` groups, fifteen of their twenty-eight pages.
Rewrites six notes, amends four ledger rows and adds two paragraphs to trap 9.
**No pixel moves and no list changes.**

## What this round was asked and what it found

672 closed with an instruction rather than a seventh idea:

> **So the criterion a next round should reach for is not a seventh — it is the sixth pointed at
> those eight.** … Twenty-eight pages of that stand in front of anybody needing a seventh idea.

The eight are eight and the twenty-eight are twenty-eight; that half of 672's account is exact.
What it did not know — because pointing an instrument at a population is the only way to find out —
is that the middle bucket is **not one bucket**. Under measurement its members separate into three
outcomes, and only one of them is the one 672's own group turned out to be:

| | |
|---|---|
| the note's mechanism accounts for the failing bound, once its own figures are put in the gate's units | `SUBPIXEL_IMAGE`, `SHARED_JBIG2_DECODER` (4 of its 7), `REFERENCES_DREW_NOTHING` |
| the mechanism accounts for it, but only an *ablation* could show that — no arithmetic converts | `LINK_BORDER`, `REFERENCE_GLYPH_WIDTHS` |
| the mechanism accounts for **none** of the failing bound, and a different clause owns it | `NEGATIVE_LINE_WIDTH` |

Two groups were not taken and are named as owed: `SUBSTITUTED_FONT` (8 pages) and
`DEVICE_CMYK_CONVERSION` (5).

## The general finding, which is worth more than any of the six

**A note's figures and the gate's figures are usually in different units, and the conversion is
short.** Five of the six notes here priced a real mechanism in ink, in coverage, in column
positions or in a perimeter, and in four of those five the arithmetic that turns it into one of the
gate's four numbers is a line long — and lands on the digit. It had not been written down once.

`raster_compare` divides by **width × height × four channels** and sums the absolute difference over
all four. That single fact is what every conversion here turns on, and it is what two of the notes
had wrong:

- a mark ours paints and a reference does not costs `Δink × 255 × 3 ÷ (w × h × 4)` in the mean,
  because three channels differ and both rasters are opaque — the factor of ¾ that makes
  `CONTRADICTED_SHARED_JBIG2_DECODER`'s ink table print the gate's own mean;
- a coloured stroke costs `perimeter × 510 ÷ (w × h × 4)` where the colour differs from white in two
  channels — 5.23 on `file_url_link.pdf`, against `poppler`'s 5.2275 measured against its own
  borderless render. `CONTRADICTED_LINK_BORDER` had **6.97**, from dividing by 10 000 pixels and
  averaging three channels;
- a differing *fraction* counts channels rather than pixels, so 180 columns of one row is 1.35
  percentage points and not 1.80. `CONTRADICTED_SUBPIXEL_IMAGE`'s page misses its bound by 1.16, so
  the factor of ¾ is the whole question.

**And `rank_the_contradicted` prints bounds, not levels of 255.** `Distance::of` reduces each
comparison to the largest of its three ratios *against the bounds the page was held to*. Two notes
described its output as levels — `CONTRADICTED_SHARED_JBIG2_DECODER`'s 28.91, which is that page's
worst tile of 144.56 over a bound of 5.00, and `CONTRADICTED_LINK_BORDER`'s 5.39/5.81/5.91, which
are neither levels nor what the gate prints now. A ranking whose unit is misread is a ranking that
cannot be checked against anything, which is why no sweep had caught either.

## Where the mechanism owns everything and the note could not say so

`CONTRADICTED_REFERENCES_DREW_NOTHING` is the limiting case and states the principle for the rest.
Where a voting reference returns a raster that is constant, the comparison has no second operand:
the mean is `255 × (1 − our own mean channel value)`, and the other three numbers are likewise
statistics of our render alone. On `issue11549_reduced.pdf` that is 12.718 against a printed 12.72;
on `issue11740_reduced.pdf` 13.672 against 13.67.

`CONTRADICTED_SHARED_JBIG2_DECODER` is the same thing four times over, and the identity is exact on
all four measurements at once. All seven of our rasters of that family are byte-identical; ours
against a synthetic 399 × 400 white sheet is **mean 13.12, worst tile 144.56, differing 5.15%, ssim
0.8990**, which is the whole verdict line the gate prints for the four pages whose voting pair
decoded nothing. **No renderer that draws this image could meet any of those bounds.** On the other
three of the seven `jbig2dec` returns ink rather than silence, and there the ink table stops being
an identity: the gate's mean exceeds ¾ of the ink *difference* by 1.4×, 1.7× and 4.9×, because the
ink is displaced rather than added and an absolute difference counts a moved mark twice.

## Where only an ablation could answer, and what its control has to be

Two mechanisms have no closed form — a glyph moved four pixels is not an ink figure — and for those
the instrument is 672's: **edit the document so that one named mechanism cannot act, re-measure, and
check that what should not move does not.**

`CONTRADICTED_LINK_BORDER`'s three pages fail on mean and structural similarity, similarity by the
wider margin. Table 166 supplies the edit — "if the border width is 0, no border is drawn" — so a
§7.5.6 incremental update restating `/Border [0 0 1]` as `/Border [0 0 0]` takes the mechanism out
and changes nothing else. Against `ghostscript`, which is where the gate's "ours at worst" comes
from on all three, mean and similarity go 7.4518/0.69785 → 2.2753/0.97056, 7.0675/0.57766 →
1.7125/0.97619 and 6.2751/0.71924 → 1.3536/0.98586. **Every page clears both bounds.** The control
is that `mupdf`, `ghostscript` and `hayro` against *each other* are byte for byte identical between
the two variants, because none of the three draws a link border.

`CONTRADICTED_REFERENCE_GLYPH_WIDTHS` is the cleaner of the two, because its control is a reference
rather than a pair. Replacing `issue9915_reduced.pdf`'s
`/W [32 [719] 0 180 719 181 [878] 182 65534 719]` with `/DW 719` — the width that array assigns to
every CID the shown line uses — sends `poppler` from mean 13.64 and similarity 0.7020 to 0.7669 and
0.99677, and `mupdf` from 13.64 and 0.7032 to 0.6233 and 0.99772, while **`ghostscript` does not
move by a single digit of any of the four**. A restatement that agrees with a renderer's reading
cannot change that renderer's picture; that it changes the other two completely is the evidence
that they are failing to find this `/W` rather than reading a different one.

## Where the group's name owns the picture and not the verdict — the second in two rounds

`CONTRADICTED_NEGATIVE_LINE_WIDTH` is 672's finding again, on a page whose *whole content* is the
mark in question, which is what makes it worth an ADR rather than a line.

`issue19633.pdf`'s crop box admits one mark: a 171.51-point diagonal at 22.56°, asked for at
`-0.1 w`. §8.4.1 clips that to 0 and §8.4.3.2 makes a zero width one device pixel; the note's ladder
prices the disagreement in ink ÷ length and prices it correctly. It converts, exactly — `Δink ×
255 × 3 ÷ (252 × 161 × 4)` against `mupdf` is 0.6366, which is the mean the gate prints — **and the
mean is a bound this page meets.** What it fails is the worst tile, by 0.09 of 5.00, and structural
similarity at 2.3 times its bound.

So the sign was ablated, in the one way the clause licenses: restate `-0.1 w` as the `0` the clip
produces, which a conforming reader must render identically. `poppler` moves a third of the way.
**`mupdf` and `ghostscript` do not move at all** — byte for byte the same four measurements — and
`mupdf` is the reference the verdict is taken from. ADR 0419's ladder said why before the experiment
did: both answer a zero width and a negative one identically at this angle, `mupdf` at 0.2 of a
device pixel and `ghostscript` at 0.27.

**The clipping rule therefore owns none of what the gate fails this page on.** The one-device-pixel
minimum owns all of it, ours is the only one of the four renderers obeying it here, and the page
stays contradicted for a clause one sentence away from the one its name gives. Third round running
in which the deciding clause sat somewhere other than where the group cites it.

## What was not done, and deliberately

- **The gate was not changed.** A voting reference whose raster is constant is, on the evidence
  above, contributing no information to a verdict, and a condition that refused it a vote would be
  the honest instrument. It would also move pages between four lists at once, and trap 11's rule —
  a report is only as good as the condition it fires on — makes that a decision with its own ADR
  rather than a corollary of this one. It is owed.
- **`SUBSTITUTED_FONT` and `DEVICE_CMYK_CONVERSION` were left**, 13 of the 28 pages. The first
  carries a related defect this round found and did not act on: its `bug847420.pdf` paragraph opens
  "the head of the contradicted list ranked in *levels* — 8.65 of 255", and `rank_the_contradicted`
  ranks in bounds, but whether 8.65 was a hand measurement or a misread ranking cannot be settled
  without measuring the page, and measuring a group is what taking it means.
- **No count was incremented.** 672's five/eight/one split is superseded by measurement rather than
  by arithmetic, and the round that takes the remaining two will find its own bucket boundaries the
  same way.

## Consequences

- Six notes state what the mechanism they name is worth in the units the verdict is made of, and
  each cites this ADR, which is what keeps them off `conformance --bin overtaken`.
- Trap 9 gains the unit rule and the constant-raster identity; trap 12 keeps its own sentence about
  the metric that fails, which this round is the fourth application of.
- The §8.4.3.2, §9.7.4.3, §10.7.4 and §12.5.4 rows carry the corpus evidence and the arithmetic.
- The next round pointing the sixth criterion at this pool has two groups left and a rule for the
  first ten minutes of it: **put the note's own figures in the gate's units before reaching for a
  renderer.** Four of six were answerable that way and none of the four had been asked.
