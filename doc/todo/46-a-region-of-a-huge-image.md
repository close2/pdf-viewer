# Decoding only the part of an image that is on the screen

Status: **raised by the project owner on 2026-08-08**, on reading session 396's reduced-resolution
work: *"Can't we do better, like only decoding part of the jpeg2000? … Or is this exactly the idea
behind the reduced resolution: to display this reduced resolution when zoomed out, where we
wouldn't see the difference?"* The second half is right, and it is only half the problem.
Priority: 46 — performance, measured on the witness below and not yet priced
Corpus: 1 (`issue19517.pdf`, 211.9 Mpx); the shape is general and reaches every codec
Clauses: §7.4.9 (`JPXDecode`), §8.9.5 (image dictionaries), §10.7.4 (what a device pixel covers)
Code: `crates/pdf-render/src/paint.rs` (`Grid`, `ImageAtDeviceScale`), `crates/pdf-model/src/image.rs`,
`crates/pdf-sandbox`, and `tmp/hayro/hayro-jpeg2000` — see `doc/JPEG2000_FEEDBACK.md` §9 for the route

## The two mechanisms, and they answer different questions

JPEG 2000 offers both, and they are orthogonal rather than alternatives:

| | mechanism | answers |
|---|---|---|
| **resolution** | the wavelet's decomposition levels: decode to level *n* and stop | *zoomed out* — the whole image is on screen and the screen has fewer pixels than the image |
| **region** | tiles, precincts and packets: skip what does not intersect a rectangle | *zoomed in* — the image is larger than the screen and only part of it is visible |

**Session 396 took the first, and the owner's reading of why is exactly right.** At fit-to-window a
211.9 Mpx image lands on roughly 1 to 2 Mpx of screen; every sample decoded beyond
`Grid::for_placement`'s answer is thrown away by the reduction that follows. ADR 0210's
`ImageAtDeviceScale::samples(Grid)` already carries the obligation — *no finer than the grid asked
for* — and until session 396 the codec was never told.

**What it cannot help with is the other direction**, and that gap is real: magnify that image to
1:1 and the grid asks for all 211.9 Mpx, of which a window shows about half a percent. Since
session 486 the decode is the budget's reduced level however far in the viewer zooms (ADR 0321) —
so 1:1 shows the reduced level magnified, which is no longer a refusal and is still not the
region's own samples.

## What the witness's own codestream says, and it is the discouraging case

Read out of `doc/pdf.js/test/pdfs/issue19517.pdf`'s SIZ and COD markers:

```text
12608 x 16806 = 211.9 Mpx, 4 components
tile 12608 x 16806  ->  1 x 1 = 1 tile
COD: LRCP, 5 decomposition levels, precincts stated: no (default: the whole subband)
```

So **the two cheap forms of random access are both absent by construction**: one tile means
tile-part skipping addresses nothing, and no precinct partition means one packet per (layer,
resolution, component) covering the whole subband. A packet is the smallest unit the codestream
*indexes*; there is no offset table that says where the top-left corner's coefficients begin.

**That does not make region decoding impossible, and it is worth being precise about why**, because
"the file is not structured for it" is the answer that sounds right and stops too early. Inside a
packet the header still states each code-block's contribution length, so a decoder that walks the
header can *skip the entropy decoding* of every code-block outside the region while still reading
past its bytes — and the inverse wavelet transform has bounded support, so reconstructing a region
needs only a slightly larger region at each coarser level. That is precisely what OpenJPEG's
`opj_set_decode_area` does, so the construction is standard rather than speculative. What it costs
is: the packet headers must still be parsed in full (tag trees are cumulative), the codestream must
still be walked end to end, and the saving is the arithmetic decoding and the IDWT — which is where
the time goes, but not where the *address space* goes.

**So the honest expectation, before anybody measures**: on this witness a region decode saves most
of the *time* and little of the *allocation*, because the coefficient buffer is sized by the
region's expanded support at each level rather than by the code-blocks actually decoded. Session
396's finding is the relevant precedent — the buffer was sized by the full-resolution rectangle
whatever was asked for, and fixing that was worth 3336 MB → 115 MB.

## What this side would need, and it is a vocabulary question

`ImageAtDeviceScale::samples(Grid)` says *how fine*. A region needs *which part*, and that is a
second thing for the display list to carry — the same shape as ADR 0210's own change, one step on.
Three things to settle before writing any of it:

- **The interpreter still may not learn the device scale.** That is what makes a display list
  re-rasterisable at any zoom, and `zooming_rasterises_again_without_interpreting_again` asserts
  it. The backend knows the viewport; the interpreter must not.
- **`render-cpu` already bands a target**, and `render-gpu` bands one the device cannot draw in a
  pass, so a crop rectangle per band exists in both backends already. Whether the region a codec is
  asked for is the *band's* or the *frame's* is a real decision: per band is smaller and asks the
  codec many times, per frame asks once and decodes more.
- **A cache makes it or breaks it.** Panning at 1:1 would re-ask for an overlapping region every
  frame; without a cache keyed by the codestream's identity *and* the region, this trades a slow
  first frame for a slow every frame. `doc/todo/45` already carries the same question for the
  reduced raster, unmeasured.

## What to do first, and it is not code

1. **A census.** Thirty corpus codestreams; how many state more than one tile, how many state a
   precinct partition, and what progression order each uses. That decides whether this is one
   file's problem or a population's, and it is twenty lines of the marker parsing already written
   into this file's header block. `doc/todo/13`'s rule, and it is what made §10.5 a small change.
2. **Then price it against the alternative**, which is that a 211.9 Mpx image at 1:1 simply is a
   large decode and the answer is to do it once and cache the result. The region route only wins if
   panning is common and memory is the binding constraint; the census plus one measurement says
   which.

## Two things this item is not

- **Not JPEG 2000's alone.** A JPEG, a JBIG2 or a huge `FlateDecode` raster has the same problem
  and none of the same mechanisms — for those, "decode the region" means decode it all and keep
  less, which is a different item. If the vocabulary above is built, it should be built so that a
  codec which cannot honour a region says so and is handed the whole thing, exactly as
  `ImageAtDeviceScale` handles a grid it cannot match.
- **Not a reason to undo session 396.** Reduced resolution is the right answer for the common case
  — a page viewed at page size — it is measured at 3336 MB → 115 MB on this witness, and session
  486 committed it (ADR 0321). Region decoding is the answer for the rarer case that this program
  now serves with the reduced level magnified rather than with the region's own samples.
