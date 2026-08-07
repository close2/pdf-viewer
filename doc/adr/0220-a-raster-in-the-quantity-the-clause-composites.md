# ADR 0220 — A raster in the quantity the clause composites

Status: accepted, 2026-08-07 (session 383).

## Context

ADR 0217 took §11.5.3's device branch into the space a mask group names: a group blending in
`DeviceCMYK` or `DeviceGray` is *painted* in §10.4.2.3's grey rather than in colour, and
`pdf_render::SoftMask::value` reads that grey back unchanged. It left two residues, and
`doc/todo/23` recorded them as **one piece of work**:

1. **A colour of more than one unit of ink.** §11.5.3 puts §10.4.2.3's `min` *after* the
   compositing and a rendered channel holds `0..=1`; `/BC [1 1 1 1]` weighs 2.0. The excess was
   clamped at each colour instead. The cost had a closed form — for artwork of ink `s` at
   coverage `α` over a backdrop of ink `1 + e`, at most `(1 − α) · e` — so it lived at the
   partly covered pixels of a mask group's own marks. Five corpus documents.
2. **Colour that arrives already rasterised.** An image's samples and a shading's ramp become
   RGB before a display list can carry them, so a subtractive space is lost. Three corpus
   documents.

ADR 0217 also recorded two constructions tried and withdrawn. An **unclamped backdrop** comes
to a negative grey that `quorra_scene::Scene::mask` refuses at its own validity test. **Scaling
the group's channel** and folding the inverse into the transfer table needs *every* colour in
the group scaled — and an image's samples are not colours the interpreter sees.

So the second withdrawn construction is the answer to residue 1, and what it was missing is
residue 2. That is why the todo file said they are one piece of work, and this round took both.

## Decision

### The group's channel carries `1 − ink ÷ scale`, and the scale is the blending space's

`crate::colour::InkScale` is the divisor, and it has exactly two values because §11.6.6 decides
which one applies. That entry is "[t]he colour space into which colours shall be converted when
painted into the group", so:

- A **`DeviceGray`** group converts a colour by §10.4.2.3 *on the way in*, and that conversion
  **is** the `min`: a grey level is `1 − min(1, ink)` and its own ink is one minus that. Nothing
  painted into such a group can weigh more than one unit, so `InkScale::Unit` and the channel is
  unscaled. **The early clamp was the clause's own arithmetic here all along**, which ADR 0217
  did not distinguish and which is why `Interpreter::colour` used to report an over-inked colour
  in a `DeviceGray` group as a departure when it was not one.
- A **`DeviceCMYK`** group keeps four components until §11.5.3, and the largest ink four clamped
  components can weigh is `0.3 + 0.59 + 0.11 + 1.0`. `InkScale::Double`, which registration
  black reaches exactly.

A colour arriving from any *other* space weighs at most one unit in either, and that is ADR
0217's cancellation restated: §10.4.2.4's black generation drops out of §10.4.2.3's weights, so
an RGB or CIE-based colour taken into `DeviceCMYK` weighs `1 − (0.3 R + 0.59 G + 0.11 B)`.

`InkScale::grey_of` is what a colour is painted as and `InkScale::mask_value` is what the mask
reads back, written beside each other so the two cannot drift.

### The `min` is composed into the mask's transfer table

`mask_value` is `1 − min(1, scale × (1 − channel))`, and it is composed with Table 142's `/TR`
into the one 256-entry table `pdf_render::Transfer` already carries. Not for economy: a backend
that expresses a luminosity mask **natively** — `render-quorra` does, with a backdrop colour and
a table — computes the luminosity in a shader of its own, so an arithmetic step outside the
table would be a step the CPU oracle takes and the graphics device does not. Composed here, both
backends are handed the same 256 bytes and cannot disagree by construction. `pdf_render` needs
no new vocabulary at all; the comment in `SoftMask::value` had already said the second half of
§10.4.2.3 lives in `Transfer`, and now it does.

At `InkScale::Unit` the composed map is the identity, so a `DeviceGray` group gets the `/TR` it
was handed and nothing about it changes.

**What that costs is one rounding, and it is the price of the whole construction.** An eight-bit
channel recovers a `DeviceCMYK` group's ink in steps of `2 ÷ 255` rather than `1 ÷ 255`: at most
one more level of 255 than the raster already rounds away, against a departure the same clause
puts at up to `(1 − α) · e` — half the mask's whole range at a half-covered pixel over
registration black. `a_grey_masks_the_same_whichever_of_the_two_device_spaces_the_group_blends_in`
is where it is stated: that test used to assert byte equality between a grey painted in a
`DeviceGray` group and in a `DeviceCMYK` one, and now allows the one level every other line in
that file allows an eight-bit mask.

### `Compositing` moves to `crate::colour`, and reaches the two rasters

A colour reaches a raster by three routes and only one of them is an operator. `Compositing` was
`crate::content`'s private enum; it is `crate::colour`'s now, with one method — `paint` — and it
is threaded into `crate::image`, `crate::shading` and `crate::mesh`. §11.6.2 is why that is the
clause rather than a convenience:

> An object's source colour Cs , used in the colour compositing formula, shall be specified in
> the same way as in the opaque imaging model: by means of the current colour in the graphics
> state or the source samples in an image.

Three things follow in the image module. `ColourSpace::reduced` **does not reduce** inside a
mask group: its three fast arms exist because `to_rgb` is the identity on a device space's
components, and painting the ink is the identity on none of them — a grey `g` in a `DeviceCMYK`
group is `(1 + g) ÷ 2`. So every sample goes through the one conversion, memoised by
`Conversion` or tabulated by `palette` as any other space's would be, and only a mask group's
images pay it. The `Decode` tables and the colour-key ranges are unaffected, because
`ColourSpace::Resolved(Gray)` answers `default_decode` and `component_range` exactly as the
reduced arm did. And every question in the module that is about *opacity* or about the shape of
the samples — a stencil, an `/SMask` image, `/Mask`'s two forms, a thumbnail — passes
`Compositing::Device` explicitly, because those read no colour.

`shading::Cache`'s key gains it, for the reason the key already carried §10.7.3's resolution:
the same shading painted inside a mask group and outside it is two ramps.

### A silence ADR 0217 left behind, now reported

§11.3.5.2 applies a separable blend function to each component "expressed in additive form", and
a subtractive group's components in additive form are the complements of its ink. What this tree
paints such a group in is one *weighted average* of them — `1 − ink ÷ 2` is
`(0.3(1 − c) + 0.59(1 − m) + 0.11(1 − y) + (1 − k)) ÷ 2`, whose weights sum to 1. Source-over is
affine and passes through an average unchanged, which is what makes the construction exact; no
other blend function does, because `B` of an average is not the average of `B`.

Until ADR 0217 every `DeviceCMYK` mask group was reported for being composited in device RGB,
which covered this without naming it. That sentence now fires only for `Lab`, so the case had
become **silent** — trap 5's failure, three sessions old. `Interpreter::note_blended_luminosity`
says the part of it that is still true, on the condition the clause gives:
`InkScale::Double` and a group whose commands blend. **No corpus document states one**, checked
over all 974, and it is kept for the reason the `Lab` report is: the alternative is a silence.
A `DeviceGray` group is exact for every blend mode, because its channel *is* its one component
in additive form, and the test asserts both halves.

## Consequences

### The measurement, against the clause's arithmetic

Three fixtures, each checked by putting the old route back:

| fixture | the clause | what the old route drew |
|---|---|---|
| `0 0 0 0 k` at `/ca 0.5` over `/BC [1 1 1 1]` | `1 − min(1, ½·0 + ½·2) = 0`, level **255** | **127** |
| a `DeviceCMYK` image of cyan in a `DeviceCMYK` group | ink 0.3, mask 0.7, level **76** | **254** |
| a `DeviceCMYK` ramp in a `DeviceGray` group | `76.5 × t`, levels **2 / 40 / 75** | 3 / **66** / 123 |

The first is the closed form `(1 − α) · e` at its widest: `e` is one whole unit of ink and `α` a
half, so the two answers are 127 of 255 apart. The third is the arithmetic in the open — at a
pixel whose centre is at parameter `t` the ink is `0.3 t` and the grey level is `1 − 0.3 t`, so a
black page keeps `76.5 t` of nothing, and the three columns are that formula at `t = 0.025`,
`0.525` and `0.975` to the level.

**On the corpus the largest page is `issue13520.pdf`**, whose `/DeviceN` shadings rest on
`DeviceCMYK` inside `/DeviceGray` mask groups: 9.17% of its pixels move by up to 27.6 of 255 and
its ink goes **20.239 → 18.998 at 2×**, by `doc/todo/00`'s own recipe — a dark blob at the
right-hand end of a row of pills shrinking toward what the references draw. At the page's own
scale that closes the step-7 gap to the lightest reference from **3.804 to 2.554**. The other six
witnesses move 0 to 8 446 pixels by at most 25.8.

### What moved on the gates

Re-run before and after; this round moves pixels.

- **corpus 974 with 73 → 70 incomplete**, and no document joined. Three left because the
  departure they named is gone — `issue14297.pdf`, `issue9017_reduced.pdf` and
  `bug1703683_page2_reduced.pdf` — and the other five witnesses keep reports about §11.4.4,
  §11.4.6 and §11.6.6, which this round does not touch. Both departure sentences are gone from
  the whole corpus: 5 occurrences of the over-ink one and 3 of the rasterised-colour one, now 0
  and 0.
- **oracle 1685 → 1688 complete**, agreeing **857 → 858**, contradicted **68** and geometry 0/2
  and not comparable 9 all identical, ambiguous 749 → **751** — the last three numbers are the
  three documents that stopped reporting arriving in the bucket. Two of them arrived
  *undiagnosed* and the gate failed on it, which is `doc/todo/00`'s instrument doing its one
  remaining job: both are diagnosed now, with two ladders each.
  - `bug1703683_page2_reduced.pdf` page 1 → `AMBIGUOUS_SUBTRACTIVE_MASK_GROUP`. Ours is flat at
    5.362–5.364 across 72, 288 and 576 dpi and ends **0.007 of 255** from `poppler`'s limit;
    `mupdf` is flat 0.14 below both.
  - `issue14297.pdf` page 1 → `AMBIGUOUS_DENSE_TEXT_AT_PAPER_SIZE`. Its ink at the page's own
    scale is 1.15 *below* the lightest reference, and the ladders say that is the references'
    scan conversion of five-point type: `poppler` 10.121 → 8.754 and `mupdf` 9.840 → 8.875 from
    72 to 576 dpi, while ours rises 8.694 → 8.821 and lands between them.
- **quorra 914 / 42 / 1 / 17 unmoved, with the furthest-from-the-oracle ranking byte-identical.**
  That is the expected shape rather than luck: the gate compares two backends on *one* display
  list, and this round changed the list.
- **text 99.2% and 25 below the floor, on a denominator that grew by 402 words** — 23 641 of
  23 841 to 24 043 of 24 243. The three documents that stopped reporting became gated, and every
  one of their words is matched.
- dates 1514 of 1545, XMP 318 read and 1 refused with 3191 properties, JPEG 2000 14
  byte-identical — all unmoved.
- **conformance 505 → 507 quotations and 4948 → 4986 citations**, 208 distinct tables and 875
  ledger rows unmoved, `partial` 248 → **249** and `inapplicable` 84 → **83**.
- **workspace tests 1296 → 1301**, ten crates 1168 → 1173, eleven 1181 → 1186, twelve 1204 →
  1209.
- **`doc/todo/00`'s step 7 over all 786 ambiguous pages: the alarm is unmoved and three lines on
  the positive side are not.** Twenty at or past −1, sixteen of them documents this tree calls
  incomplete, head `issue16038.pdf` −5.642 then `issue12295.pdf` −1.712 — every negative entry
  identical to a thousandth. (That head reads −5.398 in the three-hundred-and-eighty-second's
  record and −5.642 in the *before* sweep of this one, on the same commit: it was stale before this
  round began, and `doc/todo/00` says what the candidate is.) `issue14297.pdf` stays at −1.146 and loses its `[incomplete]` label,
  which is the same number on a page that has stopped being reported and now carries a diagnosis
  instead. What moved is where the round drew: `issue13520.pdf` **+3.804 → +2.554**,
  `bug1703683_page2_reduced.pdf` +0.142 → +0.141 and `issue12798_page1_reduced.pdf` in the fourth
  decimal at +0.068. **The sweep is not the instrument that says this round is right** — a
  positive gap shrinking is ours coming down toward the lightest reference — but it is the
  instrument that says nothing stopped being drawn, and nothing did.

### And a memo whose empty slot was a valid key

`image::resolved_sample` built its `Conversion` key by seeding a tag at bit 32 and shifting left
eight bits per component, so a four-component sample pushed the tag out of the word and an
all-zero tuple keyed on **0** — which is what every slot of the table holds before anything is
put in one. `Conversion::get` answered a *hit* out of an empty table and returned
`Color::BLACK`.

It reached any image in a space that arm converts whose samples are all zero **and whose
components are exactly four** — three leave the tag at bit 56 and a wider key is never built: a
`DeviceN` of four colourants at no tint, and — the moment this round landed — every `DeviceCMYK`
image inside a mask group, where zero ink is white and a whole half of the fixture came back
masked.
Fixed by packing the bytes into the low half and leaving the tag where the comment always said
it was. **The comment was right and the code was not**, which is why it is worth the paragraph:
the line's own doc had described the key the fix produces.

### What `doc/todo/23` still owes

The fourth population is closed. Three stand, and none of them is this:

- **§11.6.6's blending space for a painted group** (4 documents). Its result is three components
  at every pixel, so nothing here applies: the linearity that makes a mask one channel is a
  property of the *reduction* to luminosity, and a painted group is not reduced.
- **§11.4.6's knockout whose shape is not its coverage** (5) and **§11.4.4's NOTE 5
  non-isolated group** (6), both about a group's shape and its backdrop rather than its colour.
