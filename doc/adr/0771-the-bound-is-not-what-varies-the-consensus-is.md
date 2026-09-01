# ADR 0771 — The bound is not what varies; the consensus is

Status: accepted, 2026-09-01. Session 844, an oracle round on `doc/todo/12`.

`Tolerance::TEXT_HEAVY::max_differing_fraction` stays at 0.05, and the reason is no longer the
one ADR 0243 gave. Both of that ADR's preconditions for separating the consensus threshold from
the judgement floor are now met; the separated floor was implemented, run over the corpus, and
**rejected on what it moves**. Three instruments are added, one claim in `pdfref` is corrected,
and no verdict, bound or pixel moves.

## 1. What was owed, and what the `ldd` had hidden

ADR 0243 measured that this bound sits below the spread of the implementations that set it —
29.4% of text-page reference pairs are outside it where its three siblings reject 0.0%, 1.2% and
0.5% — and left it in place because it does two jobs, and because the floor half could only be
derived from a pair including a non-hinting renderer, of which the only candidate (`hayro`)
shares `skrifa` with this tree.

**That requirement rested on a factual error the tree had already corrected elsewhere.**
`Tolerance::widened_to`'s comment said `ldd` puts one `libfreetype.so.6` under `pdftoppm`,
`mutool` and `gs` alike; trap 9's fifth bullet has said since session 656 that `ldd` reports a
transitive closure and `gs` was reaching FreeType through `libfontconfig`. Re-checked on this
machine: `objdump -p` names `libfreetype.so.6` in `libpoppler.so` and `libmupdf.so` and in
neither `gs` nor `libgs.so.10`, which defines 194 `FT_*` symbols with none undefined. So a pair
of references whose glyph rasterisers are *separate objects*, neither member ours, existed all
along — `ghostscript` against either of the other two — and the derivation instrument had been
averaging across that boundary because `PairKind` had two variants where the machine has three.

## 2. The three-way split, and what it says

`the_fixed_bounds_against_the_references_own_spread` now separates `poppler` v `mupdf` from
`ghostscript` v either. Text pages, each measure taken over the pairs the other three bounds
admit, as before:

| population | pairs | median | p90 | p99 | max | over the 5.00% bound |
|---|---|---|---|---|---|---|
| `poppler` v `mupdf` (one `libfreetype.so.6`) | 893 | **0.8555%** | 5.6301% | 11.2102% | 24.3306% | **14.9%** |
| `ghostscript` v `poppler`/`mupdf` (separate copies) | 2047 | **2.5011%** | 10.7809% | **12.0372%** | 28.0398% | **34.9%** |
| `hayro` v one of the three (hinting boundary) | 2652 | 3.2668% | 11.1576% | 15.7626% | 48.2066% | 39.0% |

Two controls make it a mechanism rather than a ranking of renderers:

- **Only this measure moves across the boundary.** The other three reject 0.0%, 0.7% and 0.4% of
  the sharing pair and 0.0%, 1.5% and 0.4% of the separate-copies pairs. A renderer that were
  simply worse at text would move all four.
- **On vector pages the boundary does nothing, and points the other way.** The sharing pair is
  outside the 1.00% bound on 4.6% of vector pairs and the separate-copies pairs on 1.6%.

The step from one object to two (0.86% → 2.50%) is larger than the step from two to a
non-hinting renderer (2.50% → 3.27%), so the mechanism is the *shared object* rather than hinting
as such — which is ADR 0717's per-verdict finding arriving at the level of the bound's own
derivation, on 2940 pairs of which none is ours.

## 3. The floor, derived and priced

ADR 0243's rule, unchanged and stated before any page was looked at: **the 99th percentile of the
reference-against-reference distribution**, which is where this class's other three bounds sit.
On the population above that is **12.0372%**. Neither member is ours; neither shares code with
us; and the one population that *would* flatter us, `hayro`'s, gives a larger number — so this is
the conservative choice among the derivations available, not the flattering one.

Implemented as a floor on our own judgement inside `conclude`, with `Tolerance::accepts` and
therefore consensus formation untouched, and run over the corpus:

| | before | with the floor |
|---|---|---|
| agrees | 980 | **1017** |
| contradicted | **60** | **24** |
| ambiguous | 836 | 835 |

Thirty-six pages leave `contradicted` and none arrives; the thirty-seventh mover is
`issue11403_reduced.pdf`, whose divided consensus stops dividing. Nothing else in 1945 moves,
which is what a floor-only change must do: loosening our own bound cannot un-agree a page and
cannot form a consensus, so ADR 0243's 278 arrivals are not in this experiment at all.

**Six of the thirty-six are the reason it is not taken.**

- `calrgb.pdf` pages 1, 5, 11 and 12 and `issue9940.pdf` page 1 are all of
  `CONTRADICTED_CALRGB_TO_SCREEN` — trap 9's eighth mechanism, where `mupdf` and `ghostscript`
  each turn Table 63's `/CalRGB` dictionary into an ICC profile and hand it to Little CMS while
  `poppler` and this tree evaluate §8.6.5.3 in their own code (ADR 0494). The disagreement is
  colour, over swatches, and the group's own note prices it at 4.15 of 255.
- `issue4436r.pdf` page 1 is `CONTRADICTED_SUBPIXEL_IMAGE`, whose note writes §10.7.4's rule out
  as a closed form, measures our departure from it by a §7.5.6 incremental update, and finds the
  mask owning 1.3575 of the 1.16 points by which the page misses (ADR 0499).

The generalisation is the decision. **A differing fraction is a threshold count over channels,
so a page reaches 5–12% either by a sub-pixel phase on every glyph edge or by a small colour
error over a large area** — trap 9's own arithmetic bullet has the second in closed form, where
two levels of one channel own 2.875 of `transparent.pdf`'s 3.316 points. The two mechanisms are
indistinguishable *in this measure*, so a floor over the class cannot forgive the first without
forgiving the second, however the floor is derived.

So the answer to `doc/todo/12` is not that no number could be derived. It is that **the measure
the number bounds conflates two mechanisms, and a bound cannot separate what a mechanism
separates.**

## 4. The other branch, closed by a measurement

`doc/todo/12` asked whether a conviction resting on the shared-rasteriser pair alone should be a
*different verdict*. The candidate rule was the one trap 12 already states as a per-page control:
put a reference where our render stands, and where the consensus convicts it too, the bound is
not one an independent implementation meets.

**Measured over the whole contradicted pool, that control holds on 52 of 60 pages** — the gate
prints the count now, from `between_references` and the consensus's own bound, at no cost. It
holds on the JBIG2 pages, the `CalRGB` pages, the CMYK shading pages and the link border as
squarely as on the glyph pages. ADR 0717's *32 of 32* is therefore the pool's **base rate** and
not that population's signature, and a verdict rule resting on it would acquit us wherever two
references agree for any reason whatever — including the shared decoder and the shared profile,
where trap 9's first bullets say the consensus is manufactured and say nothing at all about who
is right.

The population that survives the control is the interesting one and it is small: on **3** pages
of the pool a voting reference outside the consensus meets the bound while we do not
(`bug847420.pdf`, `issue19633.pdf`, `issue7891_bc1.pdf`), and on five more the consensus is all
three references so there is nothing excluded to ask.

## 5. The instrument that answers `widened_to`'s standing sentence

`widened_to` has asked for four hundred sessions for "a measurement of how far a *fourth*
independent rasteriser sits from the three", and `pdfium` is still not packaged. **The question a
verdict asks does not need one**, because `decide` does not take an arbitrary pair: it takes the
pairs that agree within the fixed bounds, which on a page carrying one is the *closest* pair in
the room. The bound is a selected minimum, and what a third implementation owes is not "be as
close as two implementations typically are" but "be as close to the closest pair as the excluded
one of three manages".

That is measurable from the three references alone. `substitutions_of` runs
`pdfref::triangulate_with` with a *reference* standing where our render stands, judged by the
other two, at the page's own class and `Judgement::CORPUS` — the gate's own code path, its own
bound, its own verdict — and beside it our render judged by the same pair on the same page, so
the two counts are like for like. Over the corpus's text pages:

| consensus | pages | it contradicts the reference it excludes | it contradicts us |
|---|---|---|---|
| `poppler` + `mupdf` | 758 / 759 | **69 (9.1%)**, 58 of them on the differing fraction | **39 (5.1%)**, 35 on the differing fraction |
| `mupdf` + `ghostscript` | 675 / 677 | 23 (3.4%) | 16 (2.4%) |
| `poppler` + `ghostscript` | 650 / 652 | 4 (0.6%) | 6 (0.9%) |

**The bound is not what varies; the consensus is.** The same 5% floor convicts a known-good
independent implementation on 0.6% of text pages under the one consensus whose members do not
share the FreeType object, and on 9.1% under the one whose members do — a fifteenfold difference
in the instrument's error rate with the number, the class and the corpus held fixed. That is a
stronger statement than ADR 0243's percentile and it points somewhere else: raising the floor
would loosen the bound on the two consensuses where it is working in order to fix the one where
it is not.

And the vector table carries a finding of its own, unlooked for: the `mupdf` + `ghostscript`
consensus contradicts `poppler` on **119 of 226** vector pages, where it contradicts us on 13.
Every one of our contradictions under that consensus sits beneath a 52.7% error rate on a
program nobody suspects, which is trap 9's shared-data bullet arriving as a rate rather than as
a page.

## 6. What was checked before any of it was believed

- **The substitution instrument against the gate** (trap 13). Run on `franz_2.pdf` alone, it
  reports one consensus, `poppler` + `mupdf`, contradicting *both* our render and `ghostscript`,
  each on the differing fraction and on no other measure — which is the gate's own line for that
  page, and my hand-taken `compare_rasters` reading of `ghostscript` against the pair.
- **The gate's new count against an independent one.** The 52-of-60 the gate prints was taken
  first by a Python loop over `compare_rasters` on the artefact directories, under the stricter
  reading *outside against both members*, which gave 48 of 59 — the same population, one page
  short because one line parses differently and four short because the strict reading is
  strictly smaller. Two implementations, two readings, the same conclusion.
- **The linkage claim, re-run rather than inherited** (trap 9's last bullet): `objdump -p` on
  this machine's `libpoppler.so`, `libmupdf.so` and `libgs.so.10`, and `nm -D` for the `FT_*`
  count.
- **Seven pages of `CONTRADICTED_GLYPH_EDGES` chosen by this session rather than by its note**,
  measured pair by pair: the convicting pair at 1.28–3.79% differing, every pair containing
  `ghostscript` at 6.72–10.11%, our own best against a pair member at 5.06–7.02% and always
  nearer than `ghostscript`'s, with ink conserved to 0.43 of 255 against `poppler` and `mupdf`
  while `ghostscript` sits up to 2.7 away. The group's diagnosis reproduces on pages its own
  tables never listed.

## 7. What this does not claim

That any page is drawn differently, that any verdict was wrong, or that 0.05 is *right*. The
oracle prints 980 / 60 / 836 before and after. What changed is that the derivation now separates
the population that shares a rasteriser object from the population that does not, that the
floor-only move has a measured price with six named pages in it, that the *different verdict*
branch is closed by a base rate, and that `widened_to`'s standing request has an instrument
instead of a missing renderer.

It also does not claim that `ghostscript` against `poppler` is a *fourth rasteriser*: it is the
same algorithm in two copies, differently configured, which is a weak independence and is why
the p99 it yields is a lower bound on the true cross-implementation floor rather than an estimate
of it. `hayro`'s larger figure is consistent with that and is not admissible for the reason
`Reference::independence` gives.
