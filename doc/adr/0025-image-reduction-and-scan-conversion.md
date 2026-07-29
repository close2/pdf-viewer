# ADR 0025 — Averaging a reduced image, and the clause that forbids it

Status: accepted, 2026-07-29.

## Context

Two corpus pages had been contradicted by every reference for reasons the handover recorded
as one defect:

- `firefox_logo.pdf` draws a 512x543 logo into about a hundred pixels square. Worst tile 9.97
  against a bound of 9.95 — 0.02 outside, and it sat in the not-implemented table as "Small,
  1 document" for four sessions on that evidence.
- `bug1001080.pdf` sets its text in a Type 3 font whose every glyph description is an inline
  CCITT image mask (§9.6.4 with §8.9.7): `t` is a 39x53 bitmap drawn about five device pixels
  high, so the crossbar is one source row in fifty-three. Four renderers draw `pint test` and
  `Untitled`; we drew `pinL LesL` and `UnLiLLec`.

`tiny-skia`'s bilinear filter reads four neighbours of the sampled point whatever the
reduction, and Vello's does the same, so at eleven-to-one neither ever looks at the row the
crossbar is on. **A cosmetic-looking entry and an unreadable page were the same defect**, and
only the second one got the work scheduled.

The handover proposed "a filter that averages over the area a destination pixel covers".
Reading the clause family that governs it first — which is this project's standing habit, and
was the session's spec-track item — turned that from an obvious improvement into a decision
that has to be argued.

## What the specification actually says

**§10.7.4 addresses image reduction, and requires the opposite of area averaging:**

> The region of device space to be painted by a sampled image is determined similarly to that
> of a filled shape, though not identically. The PDF processor transforms the image's source
> rectangle into device space and defines a half-open region, just as for fill operations.
> However, only those pixels whose centres lie within the region shall be painted. The
> position of the centre of such a pixel -in other words, the point whose coordinate values
> have fractional parts of one-half -shall be mapped back into source space to determine how
> to colour the pixel. There shall not be averaging over the pixel area. If the resolution of
> the source image is higher than that of device space, some source samples might not be used.

That is point sampling, stated normatively, with the averaging this ADR adopts named and
forbidden. Three things had to be established before departing from it.

**First: this tree's comments said the standard was silent here, and they were wrong.**
`Image::is_smoothed`'s doc comment and the ledger's §8.9.5.3 row both said reduction was
something "the clause does not address", meaning §8.9.5.3, which is about `/Interpolate` and
magnification. It does not address it; §10.7.4 does. Nothing in the tree had ever cited §10.7
at all. "The clause says nothing" is a licence to choose; "the clause says the opposite" is a
debt to record, and the two had been confused.

**Second: we already depart from the same subclause, and had never said so.** §10.7.4's first
rule is that "a shape shall be scan-converted by painting any pixel whose half-open square
region intersects the shape, no matter how small the intersection is", and that "the area
covered by painted pixels shall always be at least as large as the area of the original
shape". Both backends **anti-alias**, which paints a partly covered pixel partly. That has
been true since the first commit, is what every reference renderer does for a screen, and had
no clause cited anywhere near it.

**Third: §10.7.1 is what licenses all three.** Its NOTE says the algorithm is not part of the
format:

> The specifics of the scan conversion algorithm are not defined as part of PDF. Different
> implementations can perform scan conversion in different ways; techniques that are
> appropriate for one device could be inappropriate for another.

§10.7.4 describes a device that quantises coverage to whole pixels — which is what makes its
own justification, "this ensures that no shape ever disappears as a result of unfavourable
placement relative to the device pixel grid", necessary at all. A display does not quantise
coverage, and neither does this renderer.

## Decision — average, and record it as a departure rather than a reading

`Image::area_averaged` in `pdf-render` replaces each block of source samples that would share
one device pixel with their mean, before either backend's own filter runs.

The alternative readings were both considered and both rejected:

- **Implement §10.7.4 literally.** Point sampling would make `bug1001080.pdf` a coin flip per
  glyph — some `t`s with a crossbar, some without — and would alias every photograph on every
  page drawn below full size. It would also be inconsistent: honouring the image sentence
  while ignoring the fill sentence six paragraphs above it is not a reading of the clause,
  it is a preference dressed as one.
- **Leave it, and record §10.7.4 as implemented.** This was the tempting one, because the
  standard is on its side. It fails the project's actual target — "every PDF renders as its
  producer specified" — on a page whose producer specified legible text.

So the departure is taken, and its cost is written down rather than assumed away: **a producer
who relied on a particular sample surviving the reduction gets a softened version of it
instead of that sample.** A one-pixel rule in a scanned form, a dither pattern meant to
resolve at a particular zoom, a hairline in a plan drawing — all become grey rather than
black. That is the price of legibility on the other pages, and §10.7.4's sentence exists
because some devices would rather pay the other way.

## How the blocks are chosen, and the one that was wrong

The factor per axis is `floor(samples / device length)`, from the length of the image's own
edges under the placement transform, so it holds under rotation and skew. Dividing by the
floor leaves between one and two samples per device pixel, which is exactly what the backends'
four-tap filters can see — the two are complementary rather than redundant, and the residual
work stays in the rasteriser where it belongs.

The block *boundaries* are proportional rather than fixed multiples, and the first version got
this wrong in a way worth keeping:

- **Fixed multiples with a short block at the edge.** `512 / 5` gives 102 full blocks and a
  two-sample remainder, and giving that remainder a whole output cell squeezes the image into
  99.4% of the unit square. On `firefox_logo.pdf` that moved the worst tile from 9.97 to
  **14.23** — *further* from three references than doing nothing at all.
- **Proportional boundaries.** Band `i` is `[i * samples / cells, (i + 1) * samples / cells)`
  in integer arithmetic, so the bands tile the axis exactly, no two differ by more than one
  sample, and the reduced image covers the same region of the unit square as the one it
  replaces. `Bands::at`, and `the_bands_tile_their_axis_exactly` pins it.

A sub-pixel geometry error is invisible in every picture except the one that shows it, which
is why the fix has a test rather than only a comment.

Colour is averaged **premultiplied** and divided back out. Straight-alpha components carry
whatever the encoder stored under a fully transparent sample, and averaging them directly
would let that reach the page along the soft edge of every shrunken glyph.

## Where it lives, and why not in a backend

In `pdf-render`, beside `Image::is_smoothed`, for that function's reason: the CPU backend is
the correctness oracle for the GPU one, and a resampling decision made twice is a decision the
two can disagree about. Both call the same function and draw the same raster.

`headless_gpu.rs::cpu_and_gpu_agree_on_a_deeply_reduced_image` is the guard, and its first
draft did not work: it reduced a 64x64 image into an 8x4 corner of a 200x200 page, which
differs on 32 channels of 160 000 and **passed with the GPU filter removed altogether**. The
scene now draws an 800x800 image across most of the page at five-to-one across and ten-to-one
down, and removing the GPU call site fails it at mean error 6.50 against a bound of 0.5. Trap
2's rule — a test that cannot fail in the direction the defect moves is not a test of it —
applies to magnitude as well as to axis.

## Cost, measured

`crates/pdf-model/examples/callgrind_rasterise.rs` is new, because
`callgrind_interpret.rs` stops at the display list and measures a backend change as exactly
zero. Its first draft passed 4096 as `for_page`'s **total pixel** budget rather than an extent,
so every run panicked and callgrind counted the panic — four numbers that looked like a free
change and were a measurement of nothing.

Instruction counts, twenty rasterisations per figure:

| page | before | after | |
|---|---|---|---|
| ISO 32000-2 p101, no reduced image | 14.0726 G | 14.0726 G | free |
| `bug1001080.pdf`, many 8x glyph bitmaps | 338.96 M | 330.91 M | **−2.4%** |
| `firefox_logo.pdf`, one 5x logo | 515.09 M | 540.34 M | +4.9% |
| `issue19971.pdf`, one 5x 2500x1364 photograph | 3.9264 G | 4.2793 G | +9.0% |

The corpus gate is unchanged at 1.6–1.8 s over 974 documents, so the aggregate cost is below
what that gate can measure. A page whose entire content is one large photograph pays 9%, and
a page of many small reduced bitmaps is *cheaper* than before — the filter's cost is partly
refunded by the premultiply pass and the pattern allocation, which now run over the reduced
grid instead of the source one.

The inner loop uses plain arithmetic under an `#[expect]` naming its bound, rather than the
saturating form used everywhere else in this tree. Saturating arithmetic measured **+17%**
on `issue19971.pdf` against the +9% above, and the sums provably cannot overflow: a block
holds at most as many samples as the image, which `is_consistent` has bounded by a `u32`, and
each contributes under 2^16, so every `u64` sum stays below 2^48. This is the case the
handover has been describing since the seventh session — the safety habits are expensive in a
loop that runs per sample, and that is where the profile decides rather than the habit.

## Consequences

- `bug1001080.pdf`, `firefox_logo.pdf` and `french_diacritics.pdf` all agree with the
  reference consensus. The contradicted count is 103 → **100**, agreeing 673 → **676**.
- `CONTRADICTED_IMAGE_RESAMPLING` is **empty**, and is the first group in `oracle.rs` to be.
- `french_diacritics.pdf` was not in that group. It was in `CONTRADICTED_PAGE_ROUNDING`,
  because its raster is 595x842 against `poppler`'s and `mupdf`'s 596 — which is true, and was
  not what the references were disagreeing about. **The fourth time a group's name has turned
  out not to be a diagnosis of one of its members**, after Type 3 fonts, `/Rotate` and
  `alphatrans.pdf`'s gradient.
- §10.7 has six ledger rows where it had none: §10.7.4 `partial` with its three departures
  named, §10.7.1 and §10.7.2 `implemented`, §10.7.3 `partial`, and §10.7.5 **`silent`** —
  `/SA`, automatic stroke adjustment, is read nowhere and 49 corpus documents set it true.
- §8.9.5.3's ledger row and `is_smoothed`'s doc comment are corrected: they said the standard
  was silent about reduction.

## What is deliberately not done

**§10.7.5 is not reported**, and that is trap 11 rather than an oversight. 49 documents set
`/SA true`, and a report on the key's presence would move every one of them out of the
oracle's gated set for a difference most of them cannot show — which is exactly the mistake
§9.3.8's first draft made at a seventh of the scale. The condition a report needs is that a
stroke is actually painted while the parameter is in force *and* that it is thin enough in
device space for the half-pixel adjustment the clause bounds to be visible. Sizing that is the
work the row is owed.

**Reduction still happens at decode resolution, not at device resolution.** The true answer
§10.7.4 describes is per-device-pixel, and this filter works in whole source samples, leaving
a residual under two-to-one to the backends. That is a good approximation and not the thing
itself; closing the gap means the display list carrying an image and its sampling intent to
the backends, which is the same `pdf-render` change ADR 0024 named for masks at device
resolution and belongs with it.
