# 2468 — Table 13's default of 1 governs a three-component frame whatever its identifiers spell

Session 1817, a "clause" round of batch one hundred and twenty-two, 2026-10-27. ADR 2469 unused.
Status: implemented — ends the one departure of §7.4.8's row as the owner's A668 item 1 asks.
Context: ADR 1183 (the reading it ends), ADR 2439 (the re-reading that put the question to the
owner), `doc/questions/Q668` and `A668` ("Q668 7.4.8 agree"), CLAUDE.md principle 5.

## 1. The change

§7.4.8's Table 13 ranks three cases for `/ColorTransform`, and the third reads:

> If the Adobe-defined marker code (APP14) in the encoded data indicating the ColorTransform
> value is not present and this dictionary entry is not present in the filter dictionary then
> the default value of ColorTransform shall be 1 if the image has three components and 0
> otherwise.

`image::colour_transform` ranked the first two and answered `None` for the third, leaving the
decoder's own inference to decide, and `zune-jpeg` infers "already RGB" from component identifiers
spelling `R`, `G`, `B`. It now ends in the clause: `Some(components == 3)`. `decode_jpeg` already
carried the machinery for a three-component frame the decoder read as `RGB` over which the clause
says the transform applies (`ycbcr_to_rgb`, written for the entry's `1`), so the change is one
match arm and the sentences around it; a frame the decoder reads as `YCbCr` takes the path it
always took. Four components with no marker are `CMYK` to the decoder and `Some(false)` here, the
same answer by two routes.

## 2. The population

`crates/pdf-model/examples/colour_transform_census.rs` lists the images in the clause's third case
whose frame has three components and `R`, `G`, `B` identifiers (it read the identifiers already,
and now prints each such image). Over the 974 documents of `pdf.js`'s corpus: **1 image over 1
document**, `issue11931.pdf` object 5; over the 3415 documents of `veraPDF-corpus`, `corpora`,
`corpora-own` and `pdfa`: **0**. The image is the 256 × 16 band the oracle's list already held
(`AMBIGUOUS_JPEG_COMPONENT_IDS`).

## 3. What moved

Against HEAD's six arms (`cpu`, `gpu` and `compute` at 1× and at 4×, the page digests by page,
968 pages at 1× and 965 at 4× in each lane): **exactly one page moved in each of the six,
`issue11931.pdf`, and no other**. Drawn before and after at 1× by `render_at` (an export of HEAD
against the round's), the band the 256 × 16 image fills went from the grey dome HEAD drew to a
magenta bar with a green arc — 83 602 of the page's 921 600 pixels differ, 9.07 %, the band's
1146 × 72 and the arc's margin — which is what the clause's `YCbCr` to `RGB` gives the samples
and what `ghostscript` paints. The oracle and the raster still agree on the new frame: mean
error 0.0000 at 4× and 0.0003 of 255 at 1× in every lane, where HEAD's was 0.0000 at both, and
`render-raster`'s corpus gate passes in all six arms. `raster_golden.tsv` holds one new line,
this page's, its raster and display-list digests (the extent column is unchanged), which the gate's own update mode named as the one move in 974.

## 4. What stays

- The page stays on `AMBIGUOUS_JPEG_COMPONENT_IDS`: three references still read the identifiers
  and `ghostscript` and this tree read the clause. Per principle 5 that disagreement is a question
  taken to the specification, which answered it, and it is not a target.
- A marker and an entry still outrank the default and the identifiers (ADR 1183 §§1–2 unchanged).
- The decode path of every other frame is untouched: a frame the decoder reads as `RGB` was never
  eligible for the banded direct path (it is for frames the decoder converts itself), so the
  transformed frame costs one pass of `ycbcr_to_rgb` over one image and no scheduling change.

## 5. The row

§7.4.8: `departed` to `implemented`, its note rewritten to say the clause is executed throughout,
the census and the fixture named, and the owner's question and answer cited. The test is
`tests/colour_transform.rs::with_neither_the_clauses_default_of_one_governs_whatever_the_identifiers_spell`,
which asserts the transformed pixel (derivable from ITU-T T.871's inverse, within one count) and
that the identifiers change nothing.
