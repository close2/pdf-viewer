# ADR 0418 — The library had two answers, and the mask was blamed for five hundred sessions

Status: accepted, 2026-08-18. Session 583. Asks `tiny-skia` for its high-precision raster pipeline
on every paint `render-cpu` builds, which makes §11.3.6's weighted average exact at every one of
the 256 values a mask can take; empties `CONTRADICTED_MASK_QUANTISATION` with the correction
written under its own name; amends §11.3.6's ledger row, `render-quorra`'s `TURNED_TOLERANCE` and
one knockout expectation in `pdf-model`.

## How the group was chosen, and why the tell was in its own note

`doc/HANDOVER.md`'s trap 1 says a contradicted page's group names a hypothesis rather than a
diagnosis, ten for ten, and that **a group whose note names another group's mechanism is the
cheapest tell there is**. `CONTRADICTED_MASK_QUANTISATION` held one page and its note held two
claims:

- that the verdict arises because "`mupdf` and `ghostscript` are within one level of *each other*,
  the bound derived from them is a mean of 1.11 — which our mean of 1.25 exceeds". That is
  `CONTRADICTED_TIGHT_CONSENSUS`'s mechanism, stated in that group's own words.
- that the level itself "comes from the mask being quantised: `tiny_skia::Mask` holds one byte per
  pixel and a GPU texture holds no more". **No line under it measured that**, and the group's whole
  name rests on it — as does the fix it declines to make, "a mask raster of floats, which costs
  four times the memory of every mask on every page".

Two claims in one note, one of them another group's and one of them unmeasured, is what a round
should read first.

## The page, and the arithmetic the note never did

`smask_luminosity_oob_transfer.pdf` is 500 × 300 and eight operators long: `0.95 0.95 0.95 rg`
over the whole page, then `/GS1 gs` and `0.85 0.2 0.1 rg` over the whole page, then a stroked
rectangle. `/GS1`'s `/SMask` is `/S /Luminosity`, `/BC [1]`, `/TR` a type 2 function
`0.25 + 0.5·x`, and a `/G` whose `/BBox` is `[70 80 150 140]`. So outside that box — nearly the
whole page — §11.6.5.1's rule applies: "the mask value shall be derived by transforming the BC
colour to luminosity and applying the transfer function to the result", which is `TR(1) = 0.75`.

The mask is one byte, and the byte is `round(0.75 × 255) = 191`. Put 191 through §11.3.6:

> the compositing formula collapses to a simple weighted average of the backdrop and source
> colours, controlled by the backdrop and source alpha values

with the destination the byte the grey fill actually wrote (242, so `0.949020`):

| channel | `α·Cs + (1−α)·Cb` | byte |
|---|---|---|
| R | `0.749020 × 0.85 + 0.250980 × 0.949020 = 0.874865` | **223** |
| G | `0.749020 × 0.20 + 0.250980 × 0.949020 = 0.388002` | **99** |
| B | `0.749020 × 0.10 + 0.250980 × 0.949020 = 0.313100` | **80** |

**An eight-bit mask predicts `(223, 99, 80)`, which is the closed form the note itself states.** It
does not predict our `(223, 100, 81)`. The hypothesis is refuted by its own arithmetic, and one
renderer — `hayro`, which has an eight-bit raster like everybody else — sits on the closed form
exactly.

## What it actually was

`tiny-skia` compiles each draw's raster pipeline twice: a **lowp** one carrying a pixel as four
`u16`s in `0..=255`, and a **highp** one carrying it as `f32`. It takes the first whenever every
stage of the pipeline has a lowp implementation, and a solid colour drawn through a mask always
does. A weighted average by `α` needs a division by 255 to get back from two byte factors to one,
and lowp's is

```rust
fn div255(v: u16x16) -> u16x16 { (v + u16x16::splat(255)) >> u16x16::splat(8) }
```

which is an *upper* bound on `v ÷ 255` rather than its rounding — `255·(v + 255) ≥ 256·v` for every
`v ≤ 255²`, with equality only at the ends. This path spends two of them per pixel, one in
`mask_u8` scaling the source by the mask and one in `source_over_rgba` scaling the destination by
`1 − α`, and **both biases point the same way**. Reproduced by hand out of the library's own
source, in bytes — source `(217, 51, 26)`, destination `242`, mask `191`:

```text
  div255(217·191) + div255(242·64) = 162 + 61 = 223
  div255( 51·191) + div255(242·64) =  39 + 61 = 100
  div255( 26·191) + div255(242·64) =  20 + 61 =  81
```

which is this page, arrived at from the arithmetic rather than from the raster.

`tiny_skia::Paint` has a public `force_hq_pipeline`. Swept over all 256 mask values on the same
page's arithmetic, the high-precision pipeline reproduces the closed form **at every one of them**
and the low-precision one departs by **up to two levels of 255**, always towards the backdrop.

## What it costs, because a correctness fix is still priced

Nothing measurable, and on two pages of three it is cheaper. `examples/callgrind_rasterise`, A/B in
one sitting, twenty draws a page:

| page | low precision | this clause | |
|---|---|---|---|
| ISO 32000-2 page 101 (text) | 5 568.7 M | **5 454.1 M** | −2.1% |
| `alphatrans.pdf` page 1 (transparency) | 1 977.3 M | **1 949.7 M** | −1.4% |
| `firefox_logo.pdf` page 1 (a reduced image) | 855.2 M | **860.0 M** | +0.6% |

The lowp pipeline processes sixteen pixels a stage against the highp one's eight and pays for it in
the `u16` packing and in `div255` itself. This is the rare case where `CLAUDE.md`'s tension between
speed and correctness does not arise; it is priced anyway, because the next such change might not
be free and a decision taken without a number is not a decision.

The decision lives in one named constant, `render_cpu::HIGH_PRECISION_PIPELINE`, with the argument
and the numbers on it, read by the three places this backend builds a `tiny_skia::Paint`.

## What moved, and the two things that were being flattered

The oracle: **agrees 906 → 907, contradicted 67 → 66**, nothing else moved and no page arrived.
Three other measurements moved, and each is worth more than the page.

**§11.3.6 at `α = 0.5` was landing two identical channels a level apart.**
`transparency_groups.rs::a_knockout_group_paints_only_its_topmost_element` expected `[127, 0, 128]`
for half blue over opaque red: the red channel is `0.5·0 + 0.5·255` and the blue is
`0.5·255 + 0.5·0`, the same arithmetic, and they cannot round differently. They now both give 128.
A test asserting an asymmetry that the clause forbids had stood since the mode landed.

**`render-quorra`'s turned sub-pixel ladder had a compensating bias in its thinnest rungs.** ADR
0268's substitute for a rule under the device quantum is the same rule one device pixel wide with
the width it gave up carried in the paint's **alpha** — so the thinner the rule, the larger a share
of its whole ink the upward bias was, and at the thinnest rung it cancelled the quantum outright:

```text
  45°, cpu       0.05     0.10     0.20     0.50     1.00     2.00
  low precision  -0.2%    -8.5%    -9.9%   -11.3%   -11.3%    -2.7%
  this clause   -16.8%   -11.3%    -9.9%   -11.3%   -11.3%    -2.7%
```

Nothing about the substitute moved; what moved is that the ladder now measures it. **A measurement
taken through an approximation measures the approximation too** — and a residual flat in the width,
as the right column is, is what a scan converter's quantum looks like, where one that shrinks
towards zero as the mark thins is not. `TURNED_TOLERANCE` goes 14% → 20%, which still catches the
defect the test exists for (the hairline it replaced was 29.3% short at 45°) by 1.7× rather than
2×, and the narrowing is the honest half of the correction.

**And `doc/todo/00`'s step 7 says the same thing on a real page.** Over all 786 ambiguous pages,
our ink is unchanged to a thousandth on 342 of them and the median move is 0.0035 of 255. The
alarm's head is the same names in the same order and every one of them diagnosed; the count at or
past −1 on complete documents went 4 → 3. **One page moves by more than 0.36 and it is
`issue12295.pdf`, −2.827 → −3.773**, which `examples/sub_pixel_width_census` explains without a
new hypothesis: that page states **65 859 sub-pixel strokes, every one of them 0.1366 of a device
pixel wide and near-black**, so it is ADR 0268's alpha-carried construction over two thirds of a
sheet, and it is the turned ladder's finding at page scale. The side-by-side is the argument that
this is not a regression: our ECG traces were a ghost before and are a fainter ghost now, while all
four references draw them dark, and the gap between those two pictures is `doc/todo/11`'s standing
item rather than this change's.

## What is *not* claimed

Choosing between a library's two evaluators is not implementing §11.3.6, so that row stays
`partial` and its sentence "the formula is tiny-skia's and Vello's rather than this tree's" stays
true. What this round adds to it is that the sentence named one library and two answers.

Nor is agreement with `hayro` the warrant. `hayro` shares no compositor with this tree and lands on
the closed form, which is evidence that the closed form is right; the closed form itself is
§11.3.6's own arithmetic on the file's own numbers, and it is what the code is now held to. The
page's verdict changing is a consequence and was not the target — `render-cpu/tests/soft_mask.rs`
sweeps all 256 mask values against the clause with no reference in it at all, and it fails when
`HIGH_PRECISION_PIPELINE` is flipped back, which is what establishes that it guards the decision.

## The lesson

**Eleven for eleven** on a contradicted group's name naming a hypothesis rather than a diagnosis,
and this one carried its own tell for five hundred sessions: the note's argument was another
group's, and the name's claim had no arithmetic under it. Two more general things came out of it,
and they are about instruments rather than about masks. **A tolerance written as "one level for the
quantisation" is a hypothesis wearing a number** — the test that allowed exactly one level is what
hid a two-level departure, and the sweep that allows none is what found it. And **a measurement
taken through an approximation measures the approximation**, which is how a ladder came to record
`−0.2%` for a construction that is 16.8% short.
