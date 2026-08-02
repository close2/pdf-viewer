# ADR 0151 — A stencil whose colour is a pattern

Status: accepted, 2026-08-02. Session 181. The fourth defect the ambiguous ranking named, and the
worst kind: a page drawn blank, reported complete.

## What it looked like

`issue13372.pdf` is one image on one page — hot-air balloons under a halftone screen, in a
rainbow gradient. Four reference renderers draw it. **We drew nothing**, and `corpus.rs` called
the document complete: one command in the display list, `unsupported: []`, a white page.

The oracle ranked it fifth among undiagnosed ambiguous pages, at 26.95 bounds from the nearest
reference. It could never have been contradicted: the page is a halftone reduced by two, so the
four references disagree with one another about the moiré by more than the bound, and a page that
is *entirely blank* sits in the same verdict as a page that is slightly grainy. `ambiguous` is not
a measure of how wrong a page is.

## What the file asks for

```
14 0 obj <</Subtype/Image /ImageMask true /Width 646 /Height 761 /BitsPerComponent 1
          /Filter/CCITTFaxDecode /DecodeParms<</K -1 /Columns 646>> ...>>
9 0 obj  <</PatternType 2 /Shading 10 0 R /Matrix[1 0 0 1 0 0]>>
```

A stencil mask, painted while the current colour is a **shading pattern**. §8.9.6.2 says what a
stencil does:

> Sample values in the image do not represent black and white pixels; rather, they designate
> places on the page that should either be marked with the current colour or masked out (not
> marked at all)

and §8.7.2 says what the current colour may be:

> All patterns shall be treated as colours; a Pattern colour space shall be established with the
> CS or cs operator just like other colour spaces, and a particular pattern shall be installed as
> the current colour with the SCN or scn operator

The two sentences meet in a place this tree had no representation for. A stencil is drawn as an
image whose samples carry the fill colour — `image::decode`'s `fill` parameter — and **no image
sample can carry a pattern**. `scn` with a pattern name sets `state.fill_pattern` and leaves
`state.fill` at whatever it was, which for this document is the initial black with zero alpha. So
the stencil was painted in nothing, and because every layer had done its job, no layer had
anything to report.

## The fix

The two halves are separated and recomposed out of what the display list already has. The stencil
becomes a §11.5.2 **alpha soft mask** — its marked samples are opaque and the rest are not, which
is exactly "marked with the current colour or masked out" — and the pattern fills the image's own
unit square through it. `Interpreter::stencil_through_a_pattern`.

No new command type, no new backend code, and both backends inherit it: a soft mask over a fill is
a construction they have both drawn since the fifteenth session. The pattern's `/BBox` and (since
ADR 0150) a type 1 shading's domain are composed into the clip by the same `paint_clip` any other
fill through a shading pattern uses.

## Two cases refused by name, and what refusing cost

**A tiling pattern.** It is not a paint at all — it is a content stream replayed per cell — so it
cannot be handed to a `Fill`, and replaying its cells through this mask needs the tiling machinery
to accept one. Two corpus documents ask, over five images: `issue13561_reduced.pdf` and
`bug1795263.pdf`.

**A stencil under a graphics-state soft mask**, which would be two masks on one command where
§11.6.5 makes that a composition rather than a choice. No corpus document asks.

The cost is trap 11's, paid in gated pages: the corpus's incomplete count goes **74 → 76**, and
`issue13561_reduced.pdf page 1` leaves the oracle's judged set. That is the right trade and it is
worth saying why. Before this change those five images were painted in `state.fill` — a colour a
pattern never sets, so whatever the content stream last chose, or black. A plausible-looking wrong
answer with nothing to say about itself is precisely what trap 5 forbids; a refusal that names the
clause is what this project asks for instead.

## What it bought

`issue13372.pdf page 1` draws the balloons. It stays `ambiguous`, and now for a reason that is
about resampling rather than about a blank page: a 646×761 one-bit halftone reduced into about
300×390 device pixels beats against the pixel grid, and all four renderers produce different
moirés. Ours is the closest of all six pairs — 0.0093 mean absolute error from `mupdf`, 0.0233
from `ghostscript`, 0.0461 from `poppler`, against a *best* reference pair of 0.0319 — so it joins
`AMBIGUOUS_IMAGE_REDUCTION`, whose §10.7.4 argument covers it unchanged.

Corpus 76 incomplete (was 74, both new reports named above), text unchanged at 98.2%, dates
unchanged, no oracle verdict moved but this document's and `issue13561_reduced.pdf`'s.

`image_masks.rs` gains the discriminating test: the stencil's marked cells must carry the
*gradient*, so a reader that painted them in any single colour fails the last two assertions.
Checked by breaking.

## A correction to ADR 0150

That ADR said `/Background` was "unimplemented and reported" and that a shading stating one is
"refused before it reaches here". The first half is right and the second is false — nothing reads
Table 77's `/Background` and nothing reports it, as the ledger's §8.7.4.3 row has said all along.
`issue13372.pdf`'s own shading states `/Background [0 1 1]`, which is how the error was found: an
ADR's claim about the tree is a claim like any other, and this one had never been greppped.
