# ADR 0023 — Masked images: one key, two mechanisms, and a grid to choose

Status: accepted, 2026-07-29.

## Context

ISO 32000-2 §8.9.6 names four ways an image can be made partly transparent. This tree
implemented two of them — the image's own `/ImageMask` (§8.9.6.2) and its `/SMask`
(§11.6.5.2) — and reported the other two, which share one dictionary key:

- **`/Mask` as an image** (§8.9.6.3): a second image mask saying which parts of this one are
  painted. 6 corpus first pages.
- **`/Mask` as an array** (§8.9.6.4): a range per colour component; samples inside every
  range are not painted. 2 corpus first pages, and `colorkeymask.pdf` was this project's
  standing example of a silently wrong page — three bands, the red one masked out by the
  file and drawn by us, with `unsupported: []`, until the report landed.

Both were reported as the bare string `/Mask`, without distinguishing the two forms, which is
what a report looks like when nobody has yet read the clause.

## Decision 1 — read `/Mask` once, before the samples

Colour key masking is a test on the samples themselves — "colour values before decoding with
the Decode array" — so it cannot be a post-pass over the RGBA the decoder produces: after
conversion the component values are gone, and for every space but the device ones they were
never in the raster at all. Explicit masking, by contrast, can only be applied after, and may
change the raster's dimensions.

So `/Mask` is read once at the top of `decode` into a `MaskEntry`, and each form is applied
where it belongs: the colour key travels into `unpack` as part of the `Samples` struct, and
the stencil is combined with the finished image. One place decides what the key holds, and
`unapplied_mask` — which the interpreter asks in order to report — reads the same function, so
a report cannot claim a gap the decoder does not have, or miss one it does.

## Decision 2 — combine an explicit mask and its image on the finer grid

§8.9.6.3:

> The base image and the image mask need not have the same resolution ( Width and Height
> values), but since all images shall be defined on the unit square in user space, their
> boundaries on the page will coincide; that is, they will overlay each other.

The clause states the geometry and leaves the sampling to the device, correctly: the true
answer is to composite the two at *output* resolution, which an image decoder does not know.
`pdf-model` holds one raster per image and hands it to a rasteriser that may draw it at any
scale, so combining them means choosing a grid, and there are three choices:

1. **The image's grid.** Simple, and it throws the mask's detail away. `issue4246.pdf` masks
   a 50×40 gradient with a 1000×800 stencil that spells "Image Mask Example"; on the image's
   grid the words become eight blocks.
2. **The finer of the two, per axis.** Discards nothing either raster carries. Costs the
   product of the two larger dimensions.
3. **Neither — refuse and report**, which is what `/SMask` still does for a mask of a
   different size.

This takes the second, nearest-neighbour in both directions, bounded by `MAX_MASK_GRID` at
2^24 samples with a report beyond it. Nearest-neighbour is not an approximation of a filter
here: a stencil's samples are two values with no meaningful average, and §8.9.5.3 leaves a
magnified image unsmoothed unless `/Interpolate` asks otherwise, which is the same answer for
the base image. The cost is written down rather than assumed away — where a page is drawn
larger than both grids, this differs from a device-resolution composite, and the honest fix is
the display list carrying an image and its mask separately, which is the same change
`/SMask` needs and belongs with transparency groups.

The bound exists because the grid is a product of two document-controlled numbers.
`issue16263.pdf` gives a 2×2 image a 34862×4332 `/SMask`, which on the finer grid is 604 MB
of RGBA for two distinct colours.

## Decision 3 — refuse colour key masking on a filtered image, and say so

The range test is on the samples a filter delivers, which this crate sees only where they
reach `unpack`. A `DCTDecode` or `JPXDecode` image has become RGBA before then. The clause's
own NOTE 2 names exactly that pair as the one lossy coding makes unreliable, which is a
pleasing coincidence and not the reason: the reason is that the values are not there to
compare. JBIG2 and CCITT samples *are* there, and are refused with the other two rather than
special-cased, because a colour key over one-bit samples is a stencil written the long way, no
corpus document writes one, and a rule with three exceptions is worse than a rule. All four
report.

## Decision 4 — a `/Mask` that is not an image mask is reported, not interpreted

Table 87 and §8.9.6.3 both say the entry holds an image mask, and §8.9.6.2 defines one as an
image `XObject` whose `/ImageMask` entry is true. `issue6621.pdf` writes a one-bit
`DeviceGray` *image* instead, with no `/ImageMask`, and the standard says nothing about what
that means.

The lenient reading was written first: treat it as a stencil, since one-bit single-component
samples admit no other reading, and apply §8.9.6.2's rule that a zero sample marks the page.
The page said no. `issue6621.pdf` is a court seal on a white background; under that reading
the *background* is what gets painted, and our panel came out blank beside three renderers
showing the seal. The reading those three use is §11.6.5.2's — luminosity as opacity, so white
paints — which is a different clause about a different key, and adopting it here would invert
every stencil whose author merely forgot `/ImageMask`.

So neither: the base image is drawn unmasked and the entry is named. That is visibly wrong in
a way a reader can see, which is the failure mode principle 3 prefers.

## Decision 5 — `/SMask` wins, because §11.6.4.3 says so

Reading clause 11's half of masking produced a rule that neither §8.9.6 nor Table 87 states.
§11.6.4.3, of an image's own soft mask:

> This mask, if present, shall override any explicit or colour key mask specified by the image
> dictionary's Mask entry.

and of a non-zero `/SMaskInData`, the same. So an image carrying both does not get both — the
`/Mask` is superseded by the file's own precedence, and reporting it would name a key the
document has told us not to read. `MaskEntry::Overridden` is that state, distinct from
`Absent`, so the precedence is visible in the type rather than implied by an early return.

Nothing in the corpus writes both. The rule was found by reading the clause the session's own
work cited, which is the case for the spec-driven track in one line.

## Consequences

- Five corpus documents draw completely that did not: the incomplete count falls 235 → 232
  against two arrivals from §9.3.8, and the `Image` row of the corpus gate falls 18 → 13.
- `colorkeymask.pdf` hides its red band, as four renderers do, and joins
  `CONTRADICTED_PAGE_ROUNDING` — its raster is 595 wide where `poppler`'s and `mupdf`'s are
  596, and on a page holding two coloured bands three one-pixel edges are enough. The masking
  is not in question; the heatmap shows three vertical lines and nothing else.
- `issue4246.pdf` and `issue4379.pdf` render their masked text as the four references do.
- The `/SMask`-of-a-different-size gap (§11.6.5.2, 3 documents) is now the *only* place in
  this tree where two rasters of different sizes are refused rather than combined, and the
  machinery to close it is the function this ADR describes. It was left because the
  pathological case is there rather than here — 604 MB for a 2×2 image — and because a soft
  mask carries continuous values where a stencil carries two, so the sampling question
  deserves its own answer.
