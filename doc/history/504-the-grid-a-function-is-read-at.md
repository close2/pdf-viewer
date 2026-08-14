# 504 — The grid a function is read at

**Finding.** §8.7.4.5.2 defines a type 1 shading's colour "at every point in the domain", and
this tree resolved *every point* to a 128×128 grid while building the display list —
`FUNCTION_GRID`, the same interpret-time resolution decision ADR 0210 removed for a soft mask
and ADR 0321 for a JPEG 2000 codestream, and `doc/todo/24`'s last open claimant.
`ShadingKind::Sampled` now carries the domain and a `DeferredColours` — the deferred-image
vocabulary on `Paint::Shading` — and the grid is decided once, in `pdf-render`
(`Shading::sampled_at`, `Grid::for_placement` on the domain's own placement), when a backend
knows the device. The visible change on the corpus witness is the checkerboard swatch of
`function_based_shading.pdf`: its cell edges were a three-device-pixel blur at 4× and are now
sharp, which is the clause's own discontinuity ("[t]he function need not be smooth or
continuous") landing at the device pixel. One correction along the way: the todo file cited
§8.7.4.5.3 for type 1 shadings, and the standard's §8.7.4.5.3 is *axial* — type 1 is
§8.7.4.5.2, which is what the code and ledger cite.

**Date.** 2026-08-14.
**ADR.** [0339](../adr/0339-the-grid-a-function-is-read-at.md).
**Touched.** `crates/pdf-render/src/shading.rs` (`ColourGrid`, `ColoursAtDeviceScale`,
`DeferredColours`, `Faded`, `Shading::sampled_at`; the `Sampled` variant's fields),
`crates/pdf-model/src/shading.rs` (`FunctionColours`, `MAX_FUNCTION_CELLS`; `FUNCTION_GRID`
deleted), `crates/render-cpu/src/{lib,shading}.rs` (the shader resolves at the device),
`crates/render-quorra/src/scene.rs` (`sampled_fill` likewise),
`crates/render-gpu/tests/headless_gpu.rs` (the refusal pinned by name),
`crates/test-scenes/src/lib.rs` (`sampled_shading`, `sampled_colour_at`),
`crates/render-cpu/tests/sampled_shading.rs` (new),
`crates/pdf-model/tests/shadings.rs` (the discontinuity through the whole route),
`crates/render-quorra/tests/headless_quorra.rs` (the two constructions compared at 1× and 4×),
`doc/conformance/ledger.toml` (§8.7.4.5.2, §8.9.6.3), `doc/todo/24` (deleted; README row
removed), `doc/performance.md`, `doc/JPEG2000_FEEDBACK.md`,
`doc/todo/_image-codecs-and-the-sandbox.md` (references retired), ADR 0339, this file.

## The gates

- **fmt** clean; **clippy** silent (one `unused_self` the round introduced and removed; the
  cold-build `viewer-qt` gcc notes are `doc/todo/02` §2's).
- **nextest workspace 1805 passed, 0 failed** (base 1800; the five added are the two in
  `sampled_shading.rs`, the discontinuity walk, the quorra comparison and the Vello refusal).
- **doctests** pass.
- **corpus**: gate green, 20 incomplete — the standing list; no entry appeared, left or
  changed. The two type 1 witnesses (`function_based_shading.pdf`,
  `function_based_shading_cmyk.pdf`) stay complete.
- **oracle**: green, 1794 pages — agrees 906 (862 on complete pages), contradicted 67,
  ambiguous 786, our geometry 1, reference geometry 2, not comparable 13, no render 19.
  Reference cache at 99.8% hit (the shared cache, 15 renders produced). Neither witness
  appears in the printed listings: both stay inside their diagnosed groups
  (`AMBIGUOUS_FUNCTION_SAMPLED_BY_A_REFERENCE`, the CMYK-conversion group).
- **text extraction** 99.3% (24 014 / 24 193 words, 22 documents below 90%) and the frozen
  PDFBox gate green; **dates**, **XMP**, **JPEG 2000** green.
- **quorra corpus**: green — 956 pages, 917 agree, 37 differ, 2 refused, 18 not comparable;
  no type 1 witness in the differing list. The change is scale-sensitive, so the GPU
  coverage lane at 4× (`PDFVIEWER_QUORRA_COVERAGE=gpu`, `PDFVIEWER_QUORRA_SCALE=4`) was run
  as well — 974 documents, green, ratchets off as that lane states; no type 1 witness among
  its ten furthest-from-oracle pages (the head is `issue16316.pdf` at mean 1.699).
- **conformance**: green — 7 470 citations all naming clauses the standard has, 742
  quotations all verbatim (this round's §8.7.4.5.2 and §10.7.4 blockquotes among them),
  0 cited clauses owing a review.

## The ink sweep (doc/todo/00 step 7)

Run over every artefact directory the oracle left on disk (a superset of the 786 ambiguous
pages), our ink minus the lightest non-blank reference's. **The standing negative head
reproduces**: `issue16038.pdf` −5.734 to the thousandth, then `issue12295.pdf` −2.823,
`issue14297.pdf` −1.145, `issue7821.pdf` −1.000, `jpx_smaskindata.pdf` −0.840; the rows
past −6 are the corpus's own incomplete list (unparsable fonts), as the four-hundred-and-
forty-fourth's labelled sweep recorded. The witnesses sit on the *positive* side —
`function_based_shading.pdf` p1 +1.399, the CMYK pages +3.562 and +3.032 — and that
standing surplus is their diagnosed groups' subject, not this round's: our own ink on those
three pages moved by +0.010, −0.008 and −0.056 of 255 against the before-build, measured on
the same release binary pair the pixel counts below come from. Sharpened edges redistribute
ink; they do not add it.

## The pixels that moved, and why each

Before/after on the corpus witnesses, release `render_at`, counted with `magick compare
-metric AE`:

| page | differing pixels | of the raster |
|---|---|---|
| `function_based_shading.pdf` at 1× | 1 441 | 0.30% |
| `function_based_shading.pdf` at 4× | 21 773 | 0.28% |
| `function_based_shading_cmyk.pdf` p1 at 1× | 33 | 0.04% |
| `function_based_shading_cmyk.pdf` p2 at 1× | 2 382 | 0.10% |

Every moved pixel sits where the function has detail the fixed grid interpolated: the
checkerboard's cell edges and the concentric waves' crests on page one, and the corresponding
transitions on the CMYK pages. The flat and slowly varying swatches are byte-identical — a
128-cell linear interpolation of a straight ramp is already exact, which is why the movement
is confined to the discontinuities and the curvature.

## What the trap-2 confirmation was

The fix was removed — `sampled_at` answering a fixed 128×128 grid — and three guards watched
fail before it went back: `a_sampled_shading_carries_the_functions_value_at_each_device_pixel`
(closed form, 1× and 4×), `zooming_resolves_the_shading_again_without_rebuilding_the_list`
(one list, two scales, a recording producer), and
`a_function_based_shadings_discontinuity_lands_at_the_device_pixel` (file → interpreter →
display list → backend).

## What the next round should know

- The Vello backend's refusal stands with an argument, not by default: an image inside a
  layer clipped to the shape (the mesh's and the cone's route) would genuinely fit, and was
  not taken because the shipping presentation path is quorra and a third construction is owed
  its own cross-backend evidence first. ADR 0339 prices it.
- `doc/todo/24` is gone. The vocabulary's one unmoved claimant — §8.9.6.3's explicit mask —
  lives in that clause's own ledger row, to be taken when a document asks; the one residue (a
  soft mask behind an image codec does not decode at a chosen grid) stays named in
  `eligible_for_the_device_scale` and ADR 0321.
- `MAX_FUNCTION_CELLS` (2^22) is the producer's own bound under §10.7.3's "internal limits"
  sentence; a full-page type 1 at deep zoom is the case that pays it.
