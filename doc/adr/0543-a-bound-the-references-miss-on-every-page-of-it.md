# 0543 — A bound the references miss on every page of it

Status: accepted
Date: 2026-08-23
Session: 694

Diagnoses all 63 pages of `doc/corpora/pdfbox` that the six-hundred-and-ninety-second session left
undiagnosed, in two groups: `AMBIGUOUS_TEXT_AT_DOCUMENT_SIZE` (59) and
`AMBIGUOUS_PAGE_PLACED_A_ROW_APART` (4). Empties `tests/ambiguous_undiagnosed.txt`. Corrects the
shape the queue was filed under, which was wrong about two of the four bounds.

## Context

ADR 0541 put four corpus submodules through the oracle for the first time and gated two of them.
The price it recorded, deliberately and with its argument written down, was that
`tests/ambiguous_undiagnosed.txt` — empty since the three-hundred-and-seventy-ninth session — held
63 names again, every one of them a `doc/corpora/pdfbox` page nobody had ever judged. It also left
a hypothesis about their shape, and `doc/todo/00` copied it: *62 of the 63 fail the differing
fraction and the structural similarity while sitting well inside the mean and the worst tile.*

The accusing ranking's head was 1.11 against 28.91 at the head of the contradicted ranking, so this
was a queue of measurements owed rather than of defects hidden. This round took the measurements.

## What the population is

Nineteen documents, 63 pages, all judged by `Tolerance::TEXT_HEAVY`:

- `cweb.pdf` — 22 pages of pdfTeX output at A4, ten **embedded** Computer Modern Type 1 subsets;
- PDFBox's `input/merge/` inputs — 26 pages of Acrobat Distiller Word output at US letter, whose
  text faces (`TimesNewRomanPSMT`, `ArialMT`, `Times-Roman`) are almost all **not** embedded;
- `eu-001.pdf` (3), `unencrypted.pdf` page 1, and three single pages from PDFBox issue reports;
- four `PDFBox.GlobalResourceMergeTest` names, which are one page.

**Four names, one page.** `doc/todo/00` requires asking what else in the list is the same file
before taking a name off it. `Doc01.pdf` and `Doc02.pdf` are both 9 511 bytes, both `.decoded`
variants 14 336; all four print the same four metrics to the digit, our own render is
`md5 e0a91939…` on all four and `poppler`'s `3d197096…` on all four, and their `pdftotext` readback
is one digest and is not empty. One measurement settles four names — which is
`tracemonkey.pdf`'s lesson at a smaller scale.

## The hypothesis was wrong about two bounds

Counted off the gate's own per-page lines rather than restated:

```text
  differing fraction     63 of 63 fail
  structural similarity  47 of 63
  mean                   47 of 63   — the hypothesis said none of them
  worst tile              4 of 63   — the hypothesis said none of them
```

The differing fraction is also the **worst ratio** on 59 of the 63, at 2.3 to 3.2 times its bound,
where the mean never reaches 2.6 and the worst tile never reaches 0.8. So the number that has to be
accounted for is the differing fraction, and the four worst-tile failures are a second thing that
needs its own answer.

## The differing fraction, accounted for in its own unit

`raster_compare` counts **channels** differing by more than four levels of 255, over width ×
height × four. Measured that way over the artefacts the gate has already written:

```text
  the smallest of the three reference-to-reference differing fractions
    exceeds the 5.00% bound on            63 of 63 pages     range 5.11% to 14.27%
  our worst is at or below the largest of them on   51 of 63
  medians                                  ours 11.22%    reference-to-reference 11.48%
```

**No two of the three voting references agree to this bound anywhere in this population.** A page
failing a bound that the implementations which set it also fail, on every page of the population,
is not an accusation about the renderer under test.

That is not a new result and is deliberately not presented as one: ADR 0243 re-derived all eight
fixed bounds from the corpus in the four-hundred-and-seventh session and found this one — alone
among the four — rejecting **29.4%** of reference pairs on text pages, and left it where it is for
two measured reasons. What this round adds is the same finding on a population that had no part in
setting it, produced by different generators, in a different decade, and at 100% rather than 29%
because these pages are denser text at document size than the pdf.js corpus's median.

The mean converts the same way. **On 61 of 63 our worst mean is at or below the largest
reference-to-reference mean** (median ours 6.37 against 7.90). The two exceptions are
`PDFBOX-5840-410609.pdf` pages 1 and 2, 4.64 against 4.42 and 6.63 against 5.81 — the one document
in the set whose four faces are §9.6.2.2's standard fourteen with no program embedded.

## Two camps, and which reference the gate is printing

All ten renderer pairs, over all 63 pages, in levels of 255:

```text
  the closest of the ten pairs is ours + hayro on   63 of 63
  median ours-to-hayro                               1.538
  median closest *voting* pair                       4.226
  we sit nearer a voting reference than the closest two voting
    references sit to each other on                 62 of 63
```

`doc/todo/00`'s own table for the pdf.js bucket is 1.94 and 5.39 — a ratio of 2.78 against this
population's 2.75. It reproduces to within a tenth on a corpus it was not measured on. It is **not**
evidence that we are right, and `Reference::independence` is why: `hayro` shares `skrifa` with this
tree and may not vote. What it establishes is what the verdict `ambiguous` is made of here.

The printed line on an ambiguous page is our comparison against the reference we look *least* like,
and here that is **`poppler` on 53 of the 63 and `ghostscript` on 10** — never `mupdf`. One
measurement says why: the best whole-pixel offset between our raster and `mupdf`'s is **(0, 0) on
all 63 pages**, and between ours and `poppler`'s it is **one device row down on 50 of them**.

## The closed forms

`doc/todo/00` step 6, two ladders each, with the gate's own arguments to every renderer —
`-dTextAlphaBits=4` is not optional, and without it `ghostscript`'s 72 dpi ink on `poems-beads`
reads 16.57 against the 18.25 the gate compares with, which is trap 3 inside a ladder.

```text
  cweb.pdf page 4          72 dpi     576 dpi        PDFBOX-5840-410609 p3   72 dpi   576 dpi
    ours                   14.1503    14.2305          ours                  22.9489  22.8568
    poppler                14.1894    14.2245          poppler               22.6507  22.7155
    mupdf                  14.1936    14.2461          mupdf                 22.7579  22.7390
    ghostscript            14.9504    14.2615
```

On `cweb` the two independent limits are **0.022 of 255 apart and ours at 8× lies between them**;
`ghostscript`'s 0.73 excess at the page's own scale is against its own high-resolution value. On
`PDFBOX-5840-410609` the limits agree to 0.024 and **ours is 0.13 above them at every scale**,
which is an outline difference rather than a scan-conversion one — this tree draws PDFium's Foxit
faces compiled in (ADR 0133) where the three C references read URW's off the disk.

## The four that fail the worst tile

`PDFBOX-3110-poems-beads.pdf` and its `-cropbox` twin — the same two Quartz pages, the second with
a `/CropBox` inset ten points a side, **embedded** Helvetica subsets in both. Best whole-pixel
offsets against our raster on page 1:

```text
  ours vs mupdf     2.87   best (0, 0)            ours vs hayro   2.04   best (0, 0)
  ours vs poppler  13.33   best (0, +1) → 3.77    a 72% reduction
```

The ink bounding boxes say it again: `poppler` 62..714, ours and `ghostscript` 63..714, `mupdf`
63..715, on a page whose `/MediaBox` height is 841.89 and where every renderer rasterises 842 rows.
Where the leftover 0.11 of a row goes is the question `CLAUDE.md` names as one the standard answers
nowhere — *how a fractional page becomes a whole number of pixels*. On nine-point type in
five-pixel glyph bodies a row is every baseline, which is why the worst tile fails here and on no
other page of this corpus.

The gate's line for all four is against `ghostscript`, and the ladder separates that too:

```text
  poems-beads page 1   72 dpi    288 dpi   576 dpi
    ours               16.1173   16.1288   16.1555
    poppler            16.0831   16.1666   16.1853
    mupdf              16.0772   16.1657   16.1820
    ghostscript        18.2534   16.3101   16.7208
```

`poppler` and `mupdf` converge to within **0.0033 of 255**, so this page has a limit; ours at 8× is
0.028 under it; `ghostscript` at the page's own scale is **2.07 above its own 8× value**, 12.8% of
the page's ink.

## What the specification determines

`doc/todo/00`'s second and third shapes, on two halves of the population.

**Where the fonts are embedded** — `cweb`'s Computer Modern, `poems-beads`'s Helvetica subsets,
`PDFBOX-5811-362972`'s Century, `unencrypted.pdf`'s CID TrueType — §10.7.4 determines the marks,
every renderer departs from it in its own direction at 72 dpi, and the two ladders that converge
say what the marks are. Ours is between them on `cweb` and 0.03 under on `poems-beads`.

**Where they are not** — the `merge/` documents' `TimesNewRomanPSMT` and `ArialMT`,
`PDFBOX-5840-410609`'s standard fourteen — §9.5 NOTE 5 puts the answer beyond the standard in as
many words: "some details of font naming, font substitution, and glyph selection are
implementation-dependent and can vary among different PDF processors and operating system
environments". There is no artwork to be right about, and the ladders price the choice rather than
settle it.

## Decision

Two groups rather than one, and neither of them named for the corpus they came from. The split is
by the answer rather than by the document: 59 pages whose failing bound is one the references miss
against each other on every page of the set, and 4 whose worst tile fails for a placement that can
be measured and stated.

The alternative — folding all 63 into `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE` — was rejected because
that group's note is a measurement of `tracemonkey.pdf` and `freeculture.pdf`, and adding names
under a note that does not describe them is the failure `doc/todo/00`'s "a group's diagnosis can
migrate to the group above it" section is about.

## Consequences

`tests/ambiguous_undiagnosed.txt` is empty again, and its header now says what the emptiness is a
fact about: the populations the gate currently judges, never this reader. A corpus added tomorrow
is expected to fill it.

**`doc/todo/12`'s item is unchanged and is not asked to move.** This round is a second population
saying the same thing about the differing fraction; the two reasons ADR 0243 gives for leaving the
bound where it is — that it also decides whether two references form a consensus, and that raising
it to the reference spread's 99th percentile takes the corpus to 278 newly contradicted pages —
are untouched by a corpus that agrees with the diagnosis.

**And one number in the queue's own description was wrong in the direction that flatters.** The
hypothesis said the mean was comfortable on all 63 and it fails on 47. Nothing acted on that
sentence, and the correction costs nothing; what it is worth is the reminder that a shape read off
a listing is a hypothesis in exactly the way `doc/todo/00` says, and that the round which writes
one owes the count.
