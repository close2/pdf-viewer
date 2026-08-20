# ADR 0451 — Three surfaces a page was drawn on, and none of them its own

Status: accepted, 2026-08-20. Session 615. Takes `doc/todo/03` §17's named successor — the next
chunk of the SafeDocs crawl — and fixes the three defects its seven thousand documents produced.
Amends §8.7.4.2's, §11.6.4.2's, §8.9.6.3's and §8.6.5.9's ledger rows.

## The chunk, and why these archives

§17 leaves "58 944 crawled documents unranked … in archive-sized pieces at two minutes each".
This round took **seven whole archives — `0423`, `1161`, `2268`, `3375`, `4482`, `5589` and
`6696`, 7000 documents** — none of the two session 603 ranked and none of the five session 613
did. *Which* archives is immaterial and that is ADR 0261's finding: the crawl is sorted by
SHA-256 and cut into 7933 pieces, so an archive is a hash bucket and any set of them is an
unbiased sample.

The instrument is 603's, unchanged and reused rather than rewritten: page one at 72 dpi from this
tree and from `pdftoppm`, `mutool` and `gs`, every invocation explicit about the page box
(trap 3), ranked by our ink minus the lightest live reference's with each panel's raster size
beside it.

**It was checked against 613's own numbers before it was trusted**, which is the same discipline
613 applied to 603's: this tree is at 613's commit, so the three documents that round names must
reproduce, and they do to the thousandth — `4851434.pdf` +0.127, `6327194.pdf` +0.058,
`0100223.pdf` −0.158.

## What the two ends said

**The whole positive head is this tree's, and it is three different defects.** The six deepest
rows — +232.654, +215.370, +194.095, +191.291, +175.316, +121.381 — are pages where all three
references agree within a fraction of a level and we deposit two hundred more. Opened side by
side, five are inverted photographs of a scan and one is a photographic negative of a floor plan.
Below +20 the head is 613's known shape and not ours: `poppler` alone drawing almost nothing, on
the note that round left in `doc/traps/oracle-and-references.md`.

**The negative head is one defect and then a set of reports.** `0423269.pdf` at −9.420 is a
Japanese product sheet whose two coloured backgrounds we drew as white paper, with nothing
reported at all; below it the rows carry a truncated `/FontFile2`, a `/SMask` in a three-component
space Table 143 forbids, a malformed JPEG header, a non-isolated group and a font with no outline
for any of its 3565 codes — every one of them named out loud, which is trap 5's rule working.

## 1. The surface a `sh` paints, and a lattice that moved it

`0423269.pdf`'s two backgrounds are `/PatternType 1` tiling patterns whose cell paints four
`ShadingType 4` meshes through clipped `sh` operators. Everything about them parsed: the mesh
builds, its triangles are right, its transform is right, and a red fill substituted for the `sh`
in the cell's own stream lands exactly where the blue belongs. The `sh` painted nothing.

§8.7.4.2 gives the operator no path:

> This operator does not require the creation of a pattern dictionary or a path and works without
> reference to the current colour in the graphics state.

So a display list, which fills paths, has to stand something in — and this tree stood in a
**page-sized rectangle**, which is right exactly as long as the command stays where it was drawn.
Inside a tiling pattern's cell it does not. ADR 0430 made the cell one interpretation whose marks
are *copied* to every site, and `pdf_render::Cell::repeat` displaces each copy's geometry — the
shading with it, deliberately, "so the colours have to move with the geometry or every site would
show the first site's gradient". The page rectangle is displaced by the same lattice step while
being no part of the figure. This pattern's first site is column −1, row −1, so the site whose
shading arrives on the page is the site whose rectangle has just left it, and *no* site paints.

§11.6.4.2 says what the surface should have been all along:

> For objects painted with the sh operator (8.7.4.2, "Shading operator"), the shape shall be 1.0
> inside and 0.0 outside the bounds of the shading's painti ng geometry, disregarding the
> Background entry in the shading dictionary (see 8.7.4.3, "Shading dictionaries").

Those bounds are a property of the *shading*, which is why a surface derived from them travels
with it. `pdf_render::Shading::painting_bounds` states them where the shading has them — the mesh
types, whose geometry is exactly the triangles the file writes — and `Interpreter::shading_surface`
fills that rectangle in the shading's own coordinates, grown by a sixteenth of its own extent so
that it never shares an edge with the triangles it admits. A shared edge would be counted twice,
because a clip is a coverage mask and coverages multiply; that is the arithmetic
`Interpreter::unclip_redundant` already documents for a cell's box.

**What is left is written down rather than left to be found again.** An axial shading paints an
infinite strip and a radial one an expanding cone, whatever `/Extend` says, so neither has a
rectangle of its own and both keep the page — which means a `sh` of one *inside a cell* is still
bounded by a surface the lattice displaces. Nothing in these 7000 documents does that; the
sentence is in §8.7.4.2's row so that the next document to do it is diagnosed in a minute rather
than in a round.

`0423269.pdf` −9.420 → −0.441, ours 22.109 → 31.088 against 31.529 / 31.648 / 31.930.

## 2. A preference that had been reading as a ceiling

Five of the six deepest positive rows are mixed-raster scans: a small colour layer — 391×543,
1280×1648, 1275×1650 — under a full-page 600 dpi `JBIG2Decode` or `CCITTFaxDecode` stencil in
`/Mask`. Each was **reported**, and the report is exact: `/Mask is 4758x6606 against a 391x543
image, needing a grid of 31431348 samples`. What the refusal *draws* is the base image with no
mask at all, which for that construction is a solid black page.

§8.9.6.3 puts the two rasters on one square and says nothing about a grid:

> The base image and the image mask need not have the same resolution ( Width and Height values),
> but since all images shall be defined on the unit square in user space, their boundaries on the
> page will coincide; that is, they will overlay each other.

So the grid is ours, and `combine_on_the_finer_grid` takes the finer of the two, which discards
nothing. The bound on it was `1 << 24`, and its own doc comment claimed that was "room for any
real pair" — a claim the crawl disproves five times in seven thousand, wanting 17, 31, 33, 75 and
122 million samples.

**The defect is not the level, it is that one number answered two questions.** For §11.6.5.2's
`/SMask`, exceeding it selects the *better* construction — §10.7.4's composite at device
resolution, which `SoftMaskEntry::AtDeviceScale` has carried since ADR 0210. For §8.9.6.3's
`/Mask` it selects a refusal, and always will, because a stencil is `/ImageMask true` and Table 87
says of that flag that "Mask and ColorSpace shall not be specified" — so `eligible_for_the_device_scale`,
which requires `DeviceGray`, can never say yes to one. A number that means "prefer the other
route" cannot also mean "there is no other route".

They are separate now. `PREFER_DEVICE_SCALE_ABOVE` keeps `1 << 24` and keeps its job; the ceiling
is `MAX_SAMPLES`, on the argument that **a combined grid is a raster this crate allocates**, and
one no larger than an image the crate would have decoded on its own is not larger for being a
pair. The bomb the old bound was written against is refused by the new one for the same reason it
always was — the grid is `max(w, mw) × max(h, mh)`, so 2^28 by 1 against 1 by 2^28 is two
quarter-megabyte rasters and a grid of 2^56. `issue16263.pdf`, the corpus document the old comment
named, is unaffected: its mask is `FlateDecode` `DeviceGray`, so it was already taking the
device-scale route and still does.

`2268946.pdf` +232.654 → +0.035, `2268120.pdf` +215.370 → +0.324, `4482224.pdf` +175.316 →
−0.029, `6696861.pdf` +121.381 → +0.020 — and every one of them is the reference's page when
opened, not merely its number. `3375154.pdf` +191.291 → −16.417, which is progress rather than a
fix: its 9364×13030 stencil now decodes far enough to hit `hayro-jbig2` 0.3.0's flat
10 000-instance cap, which is exactly the refusal 613 recorded on `1653119.pdf` and which upstream
has already replaced. So does `3252105.pdf` in 613's own archives, +16.771 → −6.390, a book cover
whose foreground layer is a stencil of the same kind. `doc/todo/_image-codecs-and-the-sandbox.md`
§7's release now has **three** documents of 14 000 waiting on it, which is what turns a note into
an item.

## 3. The end of the range that is dark

`2268885.pdf` at +194.095 is a floor plan drawn as a photographic negative, one command, **no
report at all**. Its one image is a `DCTDecode` under `[/ICCBased 9 0 R]`, a `scnr`-class profile
with `RGB ` data space, `XYZ ` PCS and a single `A2B0` `mft2` tag: identity matrix, two-entry
identity curves, a 13³ CLUT whose white corner is D50 and whose black corner is zero. Replacing
the space with `/DeviceRGB` draws the page at 28.4 against the references' 28.4 to 28.7.

Evaluated through the profile with black point compensation off, every input comes back within
two levels of itself. With it on, every input comes back black.

`Profile::detect_black` found the black point by pushing **every component to 1.0**. That is full
ink in `CMYK` and in a colourant space, and it is *white* in `RGB` and `GRAY` — so an additive
profile had its white point taken as its black. The guard that was meant to catch it asks only
that the colour be darker than D50 on every axis, and this profile's white corner is
(0.9628, 0.9980, 0.8219) against D50's (0.9642, 1.0, 0.8249): darker by a thousandth on each,
which passes. The stretch then divides by that thousandth and puts the whole page on black.

The application note the row already quotes says what the black is:

> aligning the darkest colour that could be described by the colour space of the data to be
> displayed with the darkest colour that the output profile for the display device (screen or
> print) can produce

— *the darkest colour the space describes*, which is at one end of the device range and not
always the same end. Both ends are evaluated now and the darker taken by luminance. That needs no
table of which header signature is additive, is unchanged for every subtractive profile (full ink
is darker, and wins), and is what the note's sentence asks for directly.

`2268885.pdf` +194.095 → +0.005. `5589678.pdf` −14.485 → −0.012 is the same fix from the other
side, and it had been silent too.

## What moved, measured on the population that found it

All **fourteen** ranked archives — this round's seven, 613's five and 603's two — were re-ranked
whole with the fixed tree and diffed row by row against the ranking that named the defects.
**15 rows of 14 000 move, and every one of them is a document one of the three fixes is about:**

| | before | after |
|---|---|---|
| `2268946.pdf` | +232.654 | **+0.035** |
| `2268120.pdf` | +215.370 | **+0.324** |
| `2268885.pdf` | +194.095 | **+0.005** |
| `3375154.pdf` | +191.291 | −16.417 |
| `4482224.pdf` | +175.316 | **−0.029** |
| `6696861.pdf` | +121.381 | **+0.020** |
| `1161651.pdf` | +59.360 | **+1.537** |
| `3252105.pdf` | +16.771 | −6.390 |
| `5589678.pdf` | −14.485 | **−0.012** |
| `0423269.pdf` | −9.420 | **−0.441** |
| `6696673.pdf` | +4.269 | **+0.093** |
| `0300214.pdf` | −2.514 | **−0.975** |
| `6696678.pdf` | −0.877 | **−0.041** |
| `0300352.pdf` | +0.840 | +0.863 |
| `1161951.pdf` | +0.580 | +0.598 |

Thirteen improve, two move by two hundredths of a level, and the two that do not close —
`3375154.pdf` and `3252105.pdf` — are the `hayro-jbig2` cap arriving from behind a bound that had
been hiding it. **Three of the fifteen are in 613's own archives** and none in 603's, which is the
second round running that a fix has reached an earlier chunk. Every other panel, ours and each
reference's, is identical.

That the list is this short was the open question for the black point change in particular, which
touches every ICC profile with a lookup table; the answer is that the guard it corrects had been
keeping almost every profile out of compensation anyway.

**No gate number moves either**, and that is the third round running the crawl has said so: no
document of the 974 states a `sh` inside a tiling cell, a `/Mask` past the old ceiling, or an
additive `A2B0` profile whose white corner misses D50. `doc/todo/00`'s step 7, re-run over all the
oracle's artefacts because this round changes what gets drawn, reproduces session 598's head and
tail to the thousandth on both sides.

## What the head still holds

Two rows below −8 are diagnosed and not taken, both silent and both trap 9's family:

- **`6696954.pdf` at −10.252** — a union newsletter whose every image is `DCTDecode` under one
  `ICCBased` space. The difference is spread over the whole page at a few levels, image and text
  alike, which is a colour conversion and not a missing mark. `poppler` and `ghostscript` share
  `liblcms2`, which is what trap 9 says to ask before reading their agreement as evidence.
- **`5589519.pdf` at −8.212** — `/DeviceCMYK` JPEGs, which is §10.4.2.5 against §10.3's ICC route
  and is the family 613 recorded on `6327765.pdf`.

And **39 rows of the 7000 produce no number**, the same three shapes 613 opened by hand: crawl
artefacts saved under a `.pdf` name, truncated files, and documents this tree refuses on a clause.

## Consequences

- A `sh` is bounded by its own painting geometry where §11.6.4.2 gives it one, so a shading a
  lattice repeats stays under the surface that admits it.
- An explicit `/Mask` is combined on the finer grid up to the same ceiling any raster gets, so a
  mixed-raster scan draws instead of turning black.
- Black point compensation asks the source space which end of its range is dark, so an additive
  profile is no longer stretched onto its own white.
- Three clause rows are amended and one `partial` row gains its fifth correction; `doc/todo/03`
  gains §18 and **51 944 crawled documents remain unranked**.
