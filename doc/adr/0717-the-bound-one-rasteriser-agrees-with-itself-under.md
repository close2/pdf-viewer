# ADR 0717 — The bound one rasteriser agrees with itself under

Status: accepted, 2026-08-28. Session 780, an oracle round on the contradicted pool's
differing-fraction tail.

The contradicted pool's largest population — the pages whose binding bound is the differing
fraction, convicted by `poppler` and `mupdf` — is measured as a population for the first time,
and on every page of it the third voting reference fails the same bound the verdict rests on.
The gate's ranking prints the count each run; no verdict, bound or pixel moves.

## 1. Why this subject

`rank_the_contradicted_by_the_bound` (ADR 0636) has printed the pool's shape since session 737:
most of the pool is furthest outside on the differing fraction, spanning about 1.0x to 29x, and
the head above the tail is fully diagnosed — the JBIG2 pages, the `DeviceCMYK` press pages, the
link borders. Session 737 recorded as owed that *the long tail just above 1.0 is a population
nobody has asked a question of as a population*, and 741 and 764 carried the item unchanged.
This round ran the instruments on a pristine baseline (983 agrees / 61 contradicted / 836
ambiguous, `PDFREF_CACHE` on the shared warm cache at a 100.0% hit rate — 6707 renders from
disk, 0 produced), read the tail off the ranking, and the tail *is* one mechanism:

- 42 of the 61 contradicted pages are furthest outside on the differing fraction;
- 32 of those are convicted by `poppler` and `mupdf` **alone** — every page of
  `CONTRADICTED_GLYPH_EDGES` (27), four of `CONTRADICTED_SUBSTITUTED_FONT`'s
  (`issue6069.pdf`, `issue6108.pdf`, `issue7580.pdf`, `bug850854.pdf`), and
  `CONTRADICTED_SUBPIXEL_IMAGE`'s one (`issue4436r.pdf`);
- the ten remaining differing-fraction pages are the colour mechanisms, convicted by `mupdf`
  and `ghostscript`, and their groups already price them (ADRs 0484, 0499, 0510).

`poppler` and `mupdf` are the one voting pair that hints its glyphs through a single
rasteriser. Re-checked on this machine's binaries rather than inherited from trap 9's bullet:
`objdump -p` on `pdftoppm` and `mutool` names `NEEDED libfreetype.so.6` in both — one shared
object, loaded by both — while `gs`/`libgs.so.10` names no FreeType at all and defines `FT_*`
symbols of its own statically linked copy.

## 2. The question, and the instrument

Trap 9's tenth mechanism — the `libfreetype` pair manufacturing a *ranking divisor* — was
measured on the ambiguous pool in ADRs 0647 and 0663 (threefold enrichment of that pair in the
*we are alone* list). On a **contradicted** page the same pair sits somewhere sharper: in
`widened_to`, because the bound the verdict rests on is derived from the convicting pair's own
spread, floored by the class tolerance. Nobody had measured, per verdict, what that pair's
agreement is made of and whether a renderer outside the pair could meet the resulting bound.

The instrument is `examples/compare_rasters` — `raster_compare::compare`, the gate's own
arithmetic — over the gate's artefacts from this round's baseline run, one named pair per row,
panels cropped top-left to the common size where they differ (what `normalise::to_common_size`
does). Calibrated at both ends before anything was read off it (trap 13):

- ours against `poppler` on `issue6069.pdf` reproduces the gate's own printed line digit for
  digit — mean 2.40, worst tile 5.33, differing 6.5550%, ssim 0.9836;
- `poppler` against `mupdf` on the same page prints differing 3.2738%, which is ADR 0606's
  recorded figure for that pair to the fourth decimal.

## 3. What the measurement says

Over all 32 pages, the differing fraction of every voting pair and of ours against each member
of the convicting pair — six `compare_rasters` invocations per page over the artefact
directory, a loop any round can rebuild in minutes and re-derive from a fresh oracle run:

| population | differing fraction |
|---|---|
| `poppler`–`mupdf`, the convicting pair | **0.00% to 4.37%, median 2.33%** |
| `poppler`–`ghostscript` | 5.32% to 13.37%, median 6.82% |
| `mupdf`–`ghostscript` | 5.35% to 13.25%, median 6.79% |
| ours, best against a pair member | median 5.70% |
| `ghostscript`, best against a pair member | median 6.75% |

Four sentences of it:

- **The two distributions do not overlap.** Every convicting pair is inside the 5.00% class
  floor of each other; every pair containing `ghostscript` is outside it. On all 32 pages the
  only two renderers that can form a consensus under this bound are the two hinting through
  one FreeType — and on `issue4061.pdf`, `issue7580.pdf` and `issue7696.pdf` that pair agrees
  to an exact printed 0.00% on the count while `ghostscript` sits at 8.59%, 5.50% and 8.64%.
- **`ghostscript` fails the bound these verdicts rest on, on every page.** Against *both*
  members of the convicting pair, 32 of 32 — so put the third voting reference where our
  render stands and the same consensus contradicts it everywhere. That is the control trap 12's
  `colors.pdf` note states for two pages (*two renderers that are not this tree fail the same
  bound*), taken over a population.
- **On 27 of the 32 it fails further than we do.** Our best-against-the-pair median is 5.70%
  against `ghostscript`'s 6.75%. The five exceptions are `issue3694_reduced.pdf`,
  `pdfbox/unencrypted.pdf` page 2, `issue7580.pdf`, `issue4436r.pdf` and `freeculture.pdf`
  page 313.
- **A count that moved at all, not an average** is what a sub-pixel phase difference produces
  (ADR 0242's arithmetic), and shared hinting is exactly what removes it between two programs:
  one rasteriser, one grid fit, the same phase decisions on both sides of the pair.

## 4. What is decided

1. **The gate prints the population.** `rank_the_contradicted_by_the_bound` counts, under its
   differing-fraction line, the pages convicted by `poppler` and `mupdf` alone, naming the
   shared rasteriser and this ADR. A population this large, resting on one mechanism, was
   visible only to a round that read 61 verdict lines by hand; a count the gate prints is one
   the sweeps can hold. Calibrated before being believed: the count printed on this tree is 32,
   which is the by-hand count off the baseline log this round started from.
2. **The notes carry the measurement where the pages live.** `CONTRADICTED_GLYPH_EDGES` — all
   27 of whose pages are in the population — carries the whole table and the control;
   `CONTRADICTED_SUBSTITUTED_FONT` names its four members and points there; trap 9's tenth
   bullet gains the contradicted-pool paragraph; `doc/todo/12` gains the per-verdict control
   beside ADR 0243's population figure.
3. **No verdict moves, no bound moves, and no rule about consensus formation changes.** The
   measurement is not evidence that our phases are right — agreement and its absence are
   evidence in one direction only, and `hayro`, which shares `skrifa` with us, is already
   refused a vote on exactly this ground. What it establishes is what the bound is made of on
   this population. Moving the bound is `doc/todo/12`, whose requirement 2 — a floor derived
   from a fourth independent rasteriser, neither member ours — this measurement does not meet
   and is not a substitute for. Disqualifying the pair outright is the move trap 9's fifth
   bullet already prices: *marking all three `Shared` for text would leave nothing to vote.*

## 5. What this does not license

- Not a loosening: the 32 pages stay contradicted, each under a note that names its bound and
  its mechanism (`--bin unpriced` reports every failing bound named both before and after this
  round).
- Not a precedent for reading a reference's failure as our acquittal: `ghostscript`'s 6.8%
  median is a fact about the bound's derivability, not about our render, and the notes say so
  in those words.
- Not a measurement of hinting as such: `-dTextAlphaBits=4` and every other invocation choice
  travels with the renderer (trap 3), so what is measured is each program as the gate invokes
  it — which is exactly what the verdicts are made of.
