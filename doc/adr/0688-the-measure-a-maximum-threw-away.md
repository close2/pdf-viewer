# 0688 — The measure a maximum threw away

**Status.** Accepted. Session 764.

`rank_the_pages_we_are_alone_on` divides our distance from the nearest reference by the closest
reference pair's distance from each other. Both halves are `outside_by_in_three_measures`: a
**maximum over three measures**, then a **minimum over comparisons**. The first of those two
reductions discards a name that `worst_ratio` keeps for the contradicted ranking — so a note could
price a page of that list as *our number is 1.83 and here is the mechanism* without ever saying
which of the three the 1.83 is. **The gate names both halves now**, with the renderers they are
measured against, and counts how many rows divide one measure by another.

## Context

ADR 0497's sixth criterion is that a contradicted page's exemption has to be priced against the
bound it actually fails, and `--bin unpriced` made that mechanical: a note arguing about a mean over
a page failing on the differing fraction has explained the picture and not the verdict. ADR 0675 is
the standing instance — a note whose mechanism predicted the mean to four decimals while the mean
was a bound the page *passes*.

That question could not be asked of the *we are alone* list at all, for a reason that is about the
instrument rather than about anybody's diligence:

- **No per-page line carries the answer.** The gate's line is our render against the consensus's
  **worst** member; this ratio is taken against its **nearest**, and over three measures rather than
  four. `--bin unpriced`'s right-hand side is that line, so its arithmetic cannot reach this list.
- **The ratio is not a ratio of like for like.** ADR 0684 already found the readable form of it —
  `freeculture.pdf` page 1 and `copy_paste_ligatures.pdf` are marked `[widened: outside]` at ratios
  under 2 precisely because our worst measure and the pair's are different measures — and left the
  consequence unstated: where the two maxima fall on different measures, **a mechanism accounting
  for the divisor need not account for the numerator's measure at all.**

Session 761 recorded the debt in one sentence: nine of the eleven priced readings on the marked head
name a mechanism without naming which of the three measures their number is.

## Decision

**`worst_ratio_in_three_measures` keeps the name, `AloneOn` carries it, and the row prints it.**

- `outside_by_in_three_measures` becomes `worst_ratio_in_three_measures(…).0`, the same split
  `outside_by` and `worst_ratio` already have over four measures, for the same reason: a ranked
  number that does not say what it is a ratio *of* is unreadable.
- `AloneOn` records, per page, the measure and reference our nearest comparison is, and the measure
  and pair the closest reference pair is. `closest_reference_pair` and `nearest_reference` are the
  two reductions written once, because three callers taking their own minimum over one list is how
  two of them eventually name different pairs.
- Each row prints `[<measure> v <reference>]` and `[<measure>, <reference> v <reference>]`, and a
  count under the list says how many rows are mixed.

The spellings are `worst_ratio`'s, which are `quoted::Measure::words`' first, so a note quoting a
row is in the vocabulary two sweeps already read.

## What reading the marked head that way found

Every row of the marked head now sits under a note that names the measure its number is taken on.
Two already did — `freeculture.pdf` page 1 and `copy_paste_ligatures.pdf`, 761's — and one named
its numerator's and not its divisor's, `bug766086.pdf`. The remaining eleven pages are seven notes,
all rewritten here. Nothing about any page's *verdict* moved and no pixel moved: every non-agreeing
per-page line is identical between the run before this round and the run after it. What moved is
what each note is able to say.

**Three shapes came out of it, and they are not the same finding:**

1. **The mechanism reaches the measure, and the note simply had not said which.**
   `AMBIGUOUS_OVERSIZED_BORDER` (mean over mean — a border is an area of ink and a mean is what an
   area moves), `AMBIGUOUS_ZERO_AREA_FILL` (similarity over similarity — the similarity orders the
   three references exactly as the ink table above it does), `AMBIGUOUS_GLYPH_COVERAGE`'s
   `endchar.pdf` (mean over mean, and its *similarity* against `mupdf` is inside the bound). Each
   now names it.

2. **The two halves are different measures, so the printed ratio understates rather than
   flatters.** `bug766086.pdf` divides a similarity by a mean: read like for like it is **14.9×**
   where the row prints 5.68×. `issue16224.pdf` is 6.2× on the similarity and 2.6× on the mean, so
   the trap-9 reading holds either way. The five `freeculture` pages are 5.3× and 5.1× on the
   similarity against a printed 3.39× and 3.30× — and on page 322 our own **mean** is 0.95 of its
   bound, *inside* it, so only the similarity puts that page on the list at all. Naming the measure
   does not excuse a page; it says what is left to explain, and on the book that is placement rather
   than ink, which the ladders answer to four decimal places and are silent about.

3. **The mechanism explains a different measure, which is what this round was sent to look for.**
   `AMBIGUOUS_STROKE_ADJUSTMENT`'s `bug1743245.pdf`. The note argues §10.7.5's single-pixel rule as
   two camps, in whole-page mean grey: ours 0.8624, `poppler` 0.8602 and `hayro` 0.8625 against
   `mupdf` 0.9537 and `ghostscript` 0.9415 — every figure of which re-takes. The row's number is a
   **structural similarity against `poppler`**, 31.43, and `poppler` is in *our* camp on the mean.
   So the mechanism cannot be what the number is.

   Priced by removal (ADR 0663's instrument, `doc/todo/00` step 1's rule). `/SA true` renamed to
   `/S1 true` in place — eight bytes for eight, so the cross-reference table still resolves, and
   Table 58's initial value for `SA` is `false`. The control: on the unedited file, freshly
   rendered, all four references and our own raster are byte-identical to the gate's cached panels.

   ```text
                          with /SA   without it
   ours vs poppler          31.43       64.34
   ours vs mupdf            44.47        2.62
   ours vs ghostscript      36.52       10.70
   ours vs hayro             2.84       50.37
   mupdf vs ghostscript      7.62        7.62   byte-identical
   ```

   **Not one reference moves by a single bit** — `poppler`, `mupdf`, `ghostscript` and `hayro` all
   render the `/SA`-free file identically to the original — while our own raster moves 18.37 of 255.
   So on this page the entry decides a pixel for this tree and for nobody else: `mupdf` and
   `ghostscript` never read it, and `poppler` and `hayro` widen a sub-pixel stroke whether the
   document asks or not. Our agreement with `poppler` and `hayro` was trap 9's *two answers to two
   different questions*, in our own camp, with the sign reversed.

   And our nearest falls to 2.62 against an unmoved divisor, so the ratio is 0.34 and **the page
   leaves that list entirely**. The whole of its rank is one clause we implement and no reference
   here conditions on. What the 31.43 *is* is the other half of §10.7.5 — the coordinate adjustment
   this tree does not implement and `poppler` does — which the same note records as a departure and
   had never joined to a number. §10.7.5's ledger row carries the measurement now.

## Consequences

- A round diagnosing this list reads a row's brackets first. Where the measures differ, the divisor's
  mechanism is an answer about the divisor and the numerator still owes one.
- A note rewritten on this list names the measure its page's number is taken on, in the gate's own
  words — the rule `--bin unpriced` states for a contradicted page, applied to a ranking.
- **Not made a sweep.** `--bin unpriced`'s right-hand side is the gate's per-page line and this
  number is not on it; a sweep over the ranking rows would need its own parser and its own
  population, and there are thirteen marked rows, all now read. It is worth building when the list
  next grows rather than as this round's second half.
- The count of mixed rows is printed rather than written down, per `CLAUDE.md`'s rule about derived
  facts.

## Alternatives considered

**Fold the differing fraction into `Distance` so both rankings use one unit.** Refused twice
already, by ADR 0643's measurement: over the complete ambiguous pool a four-measure reading names
seven pages in ten, which is `doc/todo/12`'s bound arriving as a signal. Nothing here changes that.

**Print the pair's ratio re-read on *our* measure, as a third column.** It is the number a reader
of a mixed row wants, and it is in this ADR and in each note for the pages that have one. Left off
the row because the row is already long and because the honest like-for-like question is a minimum
over *all* pairs on that measure rather than the same pair re-read — a fourth quantity, whose
argument nobody has made. `examples/compare_rasters` produces it in one command.

**Widen `Distance` to carry the name instead of adding `AloneOn`.** `Distance`'s two fields are
quoted in a hundred entries of `oracle.rs` and in `doc/todo/00`; a page recorded at "0.16 from the
nearest reference" has to stay that number, and the reference and measure belong to a *ratio* the
ranking forms rather than to the distance itself.
