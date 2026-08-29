# 0755 — One glyph, and the grid it lands on

Status: accepted.
Context: the oracle's *contradicted* pool, `CONTRADICTED_SUBSTITUTED_FONT`'s four
`pdfbox/PDFBOX-2984-rotations.pdf` pages, and `raster_compare::DEFAULT_TILE`.

## The question the round started with

Round 780 measured 32 of the pool's 61 pages down to one mechanism — the differing fraction, and a
consensus of `poppler` and `mupdf` manufactured by a shared `libfreetype.so.6` (ADR 0717, trap 9's
tenth bullet). The remaining 29 have notes; what nothing had asked is *which of them are priced in
the measure their verdict is actually taken on*, which is ADR 0497's sixth criterion as ADR 0688
sharpened it.

`--bin unpriced` cannot answer it. That sweep asks whether the note holding a page names the bound
the page fails, and over this run it reports every failing bound in the pool named, with one page
whose printed line cannot distinguish its own margin (`issue6069.pdf`, six channels of eighty
thousand — ADR 0606's finding, unchanged). *Named* is per note. Read per page, off the gate's own
lines, one group has four pages failing a measure its note never mentions:
`pdfbox/PDFBOX-2984-rotations.pdf` pages 1 to 4, failing on the **worst tile alone** at 1.10 times
their bound, under a note whose whole account of them is a cap-height ratio measured in cap rows
and whole-page ink.

Four pages of one document, falling together, on a measure nobody had priced. That is the sharpest
thing the pool had.

## What is there

**The verdict is one glyph.** `raster_compare` records where the worst tile is, and on all four
pages — ours against each of the three references, *and* between each pair of references — it is
the same tile, `(480, 64)`. What is in it is the last glyph of the line: a `registered` sign, code
`AE` under `/WinAnsiEncoding`, from a `/Helvetica` nobody embedded. Ours against `ghostscript` on
page 1, by tile, in level-pixels:

```text
  (448, 64)      5 514      mean  5.38
  (480, 64)     64 072      mean 62.57      <- the registered sign
  (512, 64)      8 740      mean  8.53
```

64 072 of the page's whole 317 158.

**And each renderer paints its own font program's area, which is a closed form and not an
agreement.** The net outline area of `registered`, read out of the two files with no renderer in
the measurement: `LiberationSans-Regular.ttf` — what `pdf_font::standard` answers `/Helvetica` with
— states 664 570.5 units² over a 2048 em, and `NimbusSans-Regular.otf`, which the three C
references resolve through this machine's fontconfig, states 228 762.3 over a 1000 em. At 50 pt
that is 396.11 px² against 571.91 px²:

```text
                             its own program says    72 dpi    576 dpi
  ours          LiberationSans        396.11         393.13     395.73
  poppler       NimbusSans            571.91         568.60     572.08
  mupdf         NimbusSans            571.91         569.83     572.06
  ghostscript   NimbusSans            571.91         574.71        —
```

At eight times the resolution every renderer is within a tenth of a percent of the area its own
face states. That is `issue15716.pdf`'s ZapfDingbats result (ADR 0499's neighbourhood) reproduced
on a second face and on a single glyph, and it is §9.5 NOTE 5 — "some details of font naming, font
substitution, and glyph selection are implementation-dependent" — with a number under it. The half
the standard *does* state is honoured on both sides: Adobe's published Helvetica advance for
`registered` is 737, `standard_metrics.rs` answers it, and the two faces' own advances are that
width to a thousandth of an em. **The layout is the document's; only the drawing differs.**

**And the mechanism owns the bound.** A §7.5.6 incremental update rewrites each content stream's
`20AE` as `2020` — the sign replaced by a space, nothing else touched — and all four renderers are
re-run at 72 dpi with the invocations `pdfref::Reference::build_command` states. The bound is twice
the widest worst tile inside the consensus, or the text class's floor of 40.00, whichever is larger:

```text
                 ours at worst   widest inside the consensus   the bound
  page 1  ships      62.57                 28.40                 56.81   contradicted
          ablated    38.27                 22.66                 45.32   inside every bound
  page 3  ships      62.01                 28.28                 56.57   contradicted
          ablated    39.93                 17.11                 40.00   inside every bound
```

Note that the ablation *tightens* the bound on pages 3 and 4 — the references agree more closely
once the glyph is gone and the class floor takes over — and the page is inside it anyway, by 0.07
of a level. Stated at the precision it was measured rather than rounded into comfort.

So this is a **vindication**: our reading is right, the mechanism is the group's own, and what the
round adds is the mechanism priced in the measure the row is ranked on.

## The finding the vindication turned up, which is about the instrument

The note also said why pages 5 and 6 of the same document *agree* while 1 to 4 do not:

> because their consensus pair happens to sit further apart and the bound derived from it is wider

That is trap 12's shape and it would have been a good answer. It is false in both halves. Measured
with the same invocations, the widest pair inside the consensus on pages 5 and 6 is **25.33**
against page 1's **28.40**, so they sit *closer* and their bound is *narrower* — 50.66 against
56.81. The bound moved against them and they agree anyway, because our own number is **35.32**
where page 1's is **62.57**, on a page carrying the same face, the same line and the same glyph.

**The whole of that difference is where the glyph falls on a grid fixed to the raster's origin.**
`raster_compare` lays its 32-pixel tiles from the origin rather than around the difference. The
registered sign occupies device columns 484 to 519 on page 1 and 526 to 561 on page 5, so on page 1
it is 28 of its 36 columns inside one tile and on page 5 it is split 18 and 18 across two:

```text
                       over the glyph's own columns   worst tile
  page 1  (480, 64)              75 004                 64 072   -> 62.57
  page 5  (512, 64)              78 212                 36 170   -> 35.32
          (544, 64)                                     28 670
```

The same glyph, the same difference to four percent, and the measure a factor of 1.77 apart — one
page contradicted and one agreeing on where a grid happened to fall.

## What was decided

**The measure is not changed, and that is the decision rather than an omission.** A sliding maximum
would remove the grid dependence and would be a *different instrument*: strictly larger on every
page, so it would move verdicts across the whole corpus in one direction, and every bound in
`Tolerance` was measured against the fixed-grid number. `doc/todo/12` already holds the standing
argument that a bound is left where it is until moving it is its own round with its own population.
What is wrong here is not the number but the reading of it, so what changes is what a reader is
told:

1. **`raster_compare::DEFAULT_TILE`'s doc comment carries the property beside the constant**, with
   this page as its witness, and states the rule it implies: *a worst tile is comparable between
   two renderings of one page, and not between two pages.* The constant's comment claimed "a single
   missing glyph dominates its tile" and that is true exactly when the glyph lands in one.
2. **A unit test pins the arithmetic** —
   `the_same_difference_reads_half_as_much_when_it_straddles_the_tile_grid`. One identical 32 × 32
   block placed at `x = 32` and at `x = 48`: the mean and the differing fraction are equal to
   1e-9 in both, and the worst tile is 191.25 against 95.625. **Calibrated against the absence of
   the property** (trap 13): run the same bodies through a 16-pixel grid, on which both placements
   are aligned, and the test fails on the halving assertion by name, reporting 191.25 where it
   wants half. It does not pass through a catch-all and it does not fail on the mean.
3. **`doc/traps/oracle-and-references.md` gains trap 26**, because the mistake this file records is
   one somebody made: the wrong explanation was in the tree, it was the plausible one, and no
   instrument could see it. What the trap asks for is one line of output — `worst_tile_at` — which
   is what turned a statistic into a place here before anything had been ablated.
4. **The group's note is rewritten** to name the failing measure, the glyph, the closed form, the
   ablation and the correction, and its cap-height section keeps its measurement with the
   distribution stated: 175.5 of the page's 402.1 px² of missing ink is the one sign, 44% of the
   deficit from one glyph in fifteen, and the 217.7 px² the cap height owns fails no bound at all.

## What this does not establish

Nothing about the other 29 pages of the pool beyond what their notes already carry. Each of them
was read against its failing bound in this round and each names it; `CONTRADICTED_DEVICE_CMYK_CONVERSION`,
`CONTRADICTED_LINK_BORDER`, `CONTRADICTED_CALRGB_TO_SCREEN`, `CONTRADICTED_REFERENCE_GLYPH_WIDTHS`
and `CONTRADICTED_TIGHT_CONSENSUS` price their mechanisms by ablation in the failing measure
already, which is why the four pages that did not were the round's subject.

And it establishes nothing at all about whether our `registered` sign is the *right* drawing. There
is no right drawing: the clause puts the outline of a substituted face beyond the standard, and the
number this round adds says only that four independent readers each drew the face they had.
