# ADR 0272 — The press was the document's, and the engine was already here

Date: 2026-08-11 (session 436)
Status: accepted

## Context

Session 433's survey of **65 944 web documents** ranked a group's blending colour space first
among everything this tree reports, at 398 documents, and split it. Two of its rows were the
same question asked twice:

| | documents |
|---|---|
| the document names its own press (§8.6.5.6's `/DefaultCMYK`, §14.11.5's output intent) | 151 |
| four components that are not `/DeviceCMYK` (an `ICCBased` space of N = 4) | 106 |

Sessions 426 and 427 had closed the `/DeviceCMYK` case: §11.3.4 composites per component, so a
three-channel rasteriser draws the page twice (ADR 0262), and a right inverse of the ink cube
converts *into* the space (ADR 0263). What was left is that this tree composited in **its** four
components and not in the ones the document named.

The round was set one question to settle before any code: **does honouring a document's own press
require evaluating a `B2A` transform, and is that an ICC dependency?**

## The clauses, and what they decide

### §8.6.5.5 states which direction the *file* must carry, not which the processor must use

For a source space the clause is explicit that `B2A` is to be ignored. For a blending space it
says the opposite about the file:

> When such a space is used as the blending colour space for a transparency group in the
> transparent imaging model … it shall have both "to CIE" ( AToB ) and "from CIE" ( BToA )
> information. This is because the group colour space shall be used as both the destination for
> objects being painted within the group and the source for the group's results.

That is a requirement on the document, and **the population honours it**: 286 of the 286
four-component blending-space profiles in the 65 944 carry both. It places no requirement on
which direction a processor evaluates.

### §14.11.5 names the other direction for exactly this device

Table 401's `/DestOutputProfile` entry is the sentence that settles it:

> The output transformation uses the profile's "from CIE" information (BToA in ICC terminology);
> the "to CIE" (AToB) information may optionally be used to remap source colour values to some
> other destination colour space, such as for screen preview or hardcopy proofing.

This processor's device is a screen. So the optional clause is the one in force, and the `A2B`
evaluator ADR 0009 wrote in `crates/pdf-model/src/icc.rs` — no dependency, `forbid(unsafe_code)`,
1171 lines — is the whole of what is needed.

### §11.7.2 says what a four-component group space *is* inside the group

> If an isolated transparency group or page has an ICCBased 'CMYK' colour space , DeviceCMYK
> shall be redefined within the transparency group to be the same as the blending colour space
> and references to the process colourants Cyan , Magenta , Yellow and Black are defined to be
> references to the corresponding colourants in the blending colour space, even where the actual
> or simulated output device is not CMYK.

So a `k` operator inside such a page states the group's own four components and needs no
conversion at all — which is what `ColourSpace::to_cmyk` already did for `DeviceCMYK`, and which
is now right for a reason rather than by accident.

### §8.6.5.7 says a colour already in the press's space is passed through

> This avoids any unwanted computational error and in the case of 4 component colour spaces
> avoids the conversion from 4 components to 3 and back to 4, a process that loses critical
> colour information.

### Annex P puts the three routes in order

Annex P is **informative** and is the standard's own algorithm for a group's blending space. Its
prose settles a reading question the normative clauses leave to inference: an isolated group with
a device blending space "first appl[ies] the default colour space mechanism", and a page group,
having no parent, inherits "from the output device, or from the output intent". So §8.6.5.6's
`/DefaultCMYK` outranks §14.11.5's output intent — which is the ranking ADR 0009 already recorded
for a colour on its way to a pixel, arrived at there from "shall" against "can suggest".

## The measurement, which decided whether this was a dependency round

`crates/pdf-model/examples/press_census.rs` reads page one of every document in the cache and
classifies the press its blending space names. Over the **65 703 of 65 944 that open**:

| | documents | profiles this tree's `A2B` evaluates | profiles carrying `B2A` |
|---|---|---|---|
| the page group's `/CS` is a four-component `ICCBased` space (§11.7.2) | **186** | 186 | 186 |
| `/DeviceCMYK` + §14.11.5's output intent | **94** | 94 | 94 |
| `/DeviceCMYK` + §8.6.5.6's `/DefaultCMYK` | **6** | 6 | 6 |
| a one-component page group (`/DeviceGray`, or an ICC profile of N = 1) | 6 | 3 | 0 |
| `/DeviceCMYK` with nothing naming the press — ADR 0263's assumed inks | 2003 | — | — |

**292 documents name a press, 286 of them with four components, and every one of those 286
profiles parses with the evaluator this tree has had since ADR 0009.** All 286 are `prtr`
output-class CMYK profiles; 285 are ICC v2 and one is v4. The presses themselves are the
industry's: FOGRA27 (33), CGATS TR 001 (17), JC200103 (15), Coated FOGRA39 (9), ISO Coated v2
300%, PSO Coated v3, U.S. Web Coated (SWOP) v2.

**So the answer to the round's first question is no on both counts.** No ICC engine is needed —
this tree is one. No `B2A` evaluator is needed either, because §14.11.5 names `A2B` for a screen
and because the conversion *into* the space is ADR 0263's right inverse, which needs only the
conversion out. Reading `B2A` as well would put two separately-built maps on one page, and a page
drawn by two colour models is the defect ADR 0262 photographed.

**A profile this tree could not parse would not be a gap either**, which is why the number above
is not the whole answer: §8.6.5.5 answers that case itself — "the colour space that shall be used
is DeviceGray , DeviceRGB , or DeviceCMYK , depending on whether the value of N is 1 , 3 , or 4 ,
respectively" — and for four components that is what `PressId::ASSUMED` already composites in.

## Decision 1 — the press is a value, and `CMYK_CORNERS` is one of them

`colour::Press` is a four-component colour space sampled onto a grid, and it carries both
directions:

- **out**, as `pdf_render::BlendingSpace`, which stops being sixteen corners and becomes a grid
  of `side⁴` samples. **At a side of two it is the sixteen corners and the arithmetic reduces
  exactly** — same accumulation, same order, same bits — so the 2003 documents that name no press
  are drawn as they were.
- **in**, as ADR 0263's search against that same grid. Every function of it — the ladder of black
  generations, the Gauss–Newton step on a fixed-black slice, the four-ink polish — now works on
  the sixteen corners of the *cell* the inks fall in, with the Jacobian scaled from the cell back
  to the ink. At a side of two the cell is the whole cube and the scale is one.

`PressId` is a `Copy` index into a fixed array of `OnceLock`s rather than an `Arc` inside
`Compositing`, and that is a shape decision with a reason: `Compositing` is `Copy`, is threaded
through every painting function in `pdf-model`, and is a `BTreeMap` key in `crate::shading`. An
index costs none of those a lifetime or an ordering, and reading a press — which happens per
colour — takes no lock. `MAX_PRESSES` is **8**, which is 8.6 MB of grids and tables, and a
document arriving after the eighth keeps the report it had before this construction existed.

## Decision 2 — seventeen samples per axis, and the residue is measured rather than assumed

A real press is not multilinear between the corners of its ink cube — that is exactly what makes
`CMYK_CORNERS` an assumption — so the grid has to be fine enough that interpolating it is the
profile. `press_census --sample` builds the grid at several sides over **the 286 presses the
population names** and compares it against evaluating the profile directly, worst case per press,
in levels of 255:

| side | median | p90 | largest |
|---|---|---|---|
| 9 | 16.34 | 18.12 | 21.60 |
| **17** | **5.99** | **11.02** | **14.52** |
| 33, on a sample of six | 1.80–4.80 | — | — |

**No feasible side reaches half a level**, and that is the round's unwelcome finding. It is a
property of the profiles rather than of the arithmetic: a v2 CMYK profile puts a steep sampled
curve on each ink *before* its own table, so a grid uniform in ink is misaligned with the shape it
samples. **Sampling in linear light is worse** — 8.62 median at side 17 against 5.99 — because it
moves the error into the bright end where a level of 255 is a smaller step. That was measured
rather than reasoned about, and it is the opposite of what the sRGB transfer curve suggests.

So seventeen is where the curve flattens against what a finer grid costs: 33 is 1.19 million
profile evaluations and 14 MB a press for about half the remaining gap.

**What the residue is measured against is what it replaces.** Compositing a page in somebody
else's four components is 48 to 51 of 255 (ADR 0251, and its fixture is half of registration black
over paper). The grid's departure from the profile is a median 5.99 and at most 14.52. So the
change is an order of magnitude, and the remaining error is written down here rather than left to
be discovered.

**The construction that would close the rest is named and not built**: per-axis input curves
beside the grid, which is what an ICC `A2B` tag *is*. That would mean teaching the backend a
second table and teaching the search the curve's derivative, and it is the thing to do if this
residue is ever the largest one on a page.

**Within a page there is still exactly one colour model**, which is ADR 0263's rule and the reason
this residue is bounded where it is. On a page that composites in a press every colour reaches the
raster through `to_cmyk` and leaves it through the grid; `to_rgb` answers only the alpha there. The
discrepancy is between a press page and a page in the same document that composites on the device,
and it is at most the 14.52 above.

## What the pictures showed

Session 426's lesson — the decisive finding was in a picture, not a count — was applied to the
fixture rather than to the corpus, because **none of the 974 pdf.js documents names a press**
(0 of 974, measured, against 292 of 65 703 on the web). So the picture is the one the clause's own
arithmetic predicts: half of registration black over paper is `[0.5, 0.5, 0.5, 0.5]` by §11.3.4
whatever the press is, and what the press decides is the colour that is.

`the_three_routes_to_a_press_all_composite_in_it` draws exactly that page three times — once with
the press named by the page group's `/CS`, once by `/DefaultCMYK`, once by an output intent — and
each pixel is the profile's own answer for `[0.5, 0.5, 0.5, 0.5]` to within two levels. The same
content with no press named is 76 of 255 in red, which is ADR 0251's number for the assumed inks,
and the two are more than eight levels apart in every channel. Putting the assumed press back
fails the test.

## What the gates said

**The 974 did not move, and that is the check rather than the hope**: no corpus document names a
press, and the assumed press's grid is the old sixteen corners bit for bit. Corpus 974 with 68
incomplete; oracle 1794 pages, 1690 complete, verdicts 905 / 68 / 786 with 1, 2, 14 and 18;
quorra 911 / 35 / 11 / 17; text 99.2% (24003/24187) and PDFBox 99.8% (14257/14281) — every one
identical. **`doc/todo/00`'s step 7 is therefore not owed**, by the same argument ADR 0269 made:
quorra compares this backend's raster page by page, so a changed CPU raster would move its
differing list.

**The web population is where the change is**, 65 944 documents before and after:

| | before | after |
|---|---|---|
| **incomplete** | 1138 | **905** |
| the document names the press its `DeviceCMYK` is | 151 | **0** |
| components that are not four this tree can sample | 106 | **5** |
| a group inside the page composites in a different space (§11.6.6) | 78 | 85 |
| a non-separable blend mode (§11.3.5.3) | 27 | 28 |
| an `/ExtGState` states Table 57's `/BG` or `/UCR` (§11.7.5.3) | 7 | 9 |

**369 blending reports become 127, and 233 documents become complete.** The three rows that grew
are documents that were reported for the press and are now reported for the next condition they
meet, which is the population narrowing honestly rather than a condition being narrowed (trap 5).
Both passes are 65 944 documents with no failure of any kind; the second is 1237 s against 848,
which is the grid and the ink table a press costs on the pages that want one.

Tests 1594 → **1599**, citations 6378 → **6450**, quotations 597 → **610**, ledger 875 rows with
`partial` 250 → **251** and `inapplicable` 83 → **82**.

## The row that stopped being inapplicable

§8.6.5.7 was `inapplicable` on the argument that "this device is an sRGB screen and every colour
is converted to it at the moment it is read, so there is no four-to-three-to-four round trip for
the clause to save". That was true while every page composited on the device. It stopped being
true the moment a page composites in a press: a `DeviceCMYK` colour on such a page *is* the
press's four components, and converting it to sRGB and back would be exactly the round trip the
clause exists to save. The row is `partial` now.

`CLAUDE.md` says a claim about the specification decays, and its standing example is the
`DeviceCMYK` → RGB conversion that turned out to be answered twice. This is the same shape one
clause over: nothing about §8.6.5.7 changed, the code around it did.

## Consequences

- **The largest single condition the web has is closed**, measured: 151 documents reported it and
  none do. The four-component row is 106 → **5**, and what is left there is a blending space that
  is not four components this tree can sample — a `/DeviceGray` page group, a `Lab` one, or four
  components with no profile behind them.
- **`doc/todo/23`'s remaining rows are §11.6.6's group-level conversion (85), a non-separable
  blend (28), Table 57's black generation (9) and those 5**, and the first of those is now the
  largest single thing this tree reports about a blending space.
- **The residue this round created is a number**: the grid's 14.52 of 255 against the profile, and
  the construction that closes it. It is written in `PRESS_SIDE`'s own doc comment, in
  `doc/todo/23` and here.
- **A press's grid is built on the launch path of a page that wants one** — 83 521 profile
  evaluations, serial, under the registry's lock — and the ink table behind the conversion in is
  built lazily on top of that, off a rayon worker, which is ADR 0269's rule kept.
