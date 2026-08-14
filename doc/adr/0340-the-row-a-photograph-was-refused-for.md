# 0340 — The row a photograph was refused for

**Status.** Accepted.

## Context

`doc/todo/03` §7 says where a corpora round should point next: the ink assertion that made
`pdf-handbuilt-test-corpus` worth three rounds, aimed at `pdfCabinetOfHorrors` or
`govdocs1-error-pdfs`, "where the files do *not* share a page and the method therefore needs a
reference again". This round took both — 78 documents — and ran our page one against `pdftoppm`,
`mutool` and `gs` at 72 dpi, every one of them told to use the crop box (trap 3), ranking **our
ink minus the lightest live reference's** the way `doc/todo/00` step 7 ranks the ambiguous bucket.

One row of 78 is not a rasterisation difference. `veraPDFHiResChangedHeight.pdf` is
**−94.953**, which is the whole page: the three references draw a photograph and we draw white.
The next largest negative is −1.719 and everything below −0.65 is a document this tree already
reports on.

The file is `veraPDFHiRes.pdf` — an intact PDF/A-1b page carrying one JPEG — with **one digit of
its `/Height` altered on purpose**, 1226 → 1227. That is the whole defect, and it is what the
directory exists to state.

## The clause, and what the code was doing instead

`decode_jpeg` refused any codestream whose frame disagreed with the dictionary, on this comment:

> The dictionary and the JPEG both state the dimensions; they must agree, because the display
> list carries the dictionary's and the samples carry the JPEG's.

The premise was true when it was written, in the commit that first drew an image, and it expired
without anyone noticing. `SamplesOnGrid` grew a `grid` field so that a `JPXDecode` codestream
decoded at a reduced resolution level could answer with the grid it was *actually* on, and the
comment beside the `Image` it builds already states the general rule: "an image occupies the unit
square whatever its resolution (§8.9.5.1), so the raster's own grid is the only honest one to
build at". A second codec needed the same field and nothing had asked.

The standard is not silent here, and it does not say what the refusal assumed. §7.4.8:

> The values of these parameters, which include the dimensions of the image and the number of
> components per sample, are entirely under the control of the encoder and shall be stored in the
> encoded data. DCTDecode may obtain the parameter values it requires directly from the encoded
> data.

So a JPEG's dimensions live in the JPEG, and a decoder has nowhere else to read them from.
§8.9.5.1 then puts the samples on the page without consulting them again — "the unit square of
user space, bounded by user coordinates (0, 0) and (1, 1), corresponds to the boundary of the
image in image space" — which is the same sentence §8.9.6.3 leans on when it lets an explicit
mask and its base image have different `/Width` and `/Height` outright: "The base image and the
image mask need not have the same resolution ( Width and Height values), but since all images
shall be defined on the unit square in user space, their boundaries on the page will coincide". **A
grid is a sampling density, not a size on the page.** A dictionary that contradicts the frame has
therefore contradicted the data about a number that decides nothing the page can see.

**And the contrast is what makes this a reading rather than leniency.** §7.4.9 states, for
JPEG 2000, precisely the constraint §7.4.8 does not state for JPEG:

> Width and Height shall match the corresponding width and height values in the JPEG 2000 data.

That refusal stays exactly where it is. What separates the two cases is the clause, not the
codec, and this tree now answers each one with what its own clause says.

## Decision

**A `DCTDecode` image is built on the grid its codestream states.** `decode_jpeg` returns that
grid alongside the samples and the component count, `samples_of`'s `DCTDecode` arm passes it
through the field a reduced JPEG 2000 decode already uses, and the dictionary's `/Width` and
`/Height` are not compared against it.

**And the disagreement is reported beside the drawing**, which is the seventh of the places this
tree reports *while* drawing and was decided by a file rather than by taste. The first version of
this change was silent, on trap 11's argument that no mark is missed. The oracle answered it
within the round: `xobject-image.pdf` — a pdf.js fixture whose dictionary says 200×100 and whose
codestream is **one red pixel** — went from refused-and-reported to drawn-and-silent, entered the
judged set for the first time, and was contradicted at 99.78 from the nearest reference. It is a
flat red rectangle where the file describes a picture, and no reader could tell.

So the test `doc/HANDOVER.md` sets for reporting while drawing is met, in both directions:
suppress the drawing and the photograph is lost; suppress the report and a page can show a flat
colour where its producer described a photograph, with nothing saying so. `image::contradicted_frame`
asks the codestream — headers only, no entropy decoding — rather than reading the dictionary
alone, for the same reason `unapplied_mask` does: a report that reads only what the file says can
outlive the gap it describes.

The three references are worth recording here precisely because they do not agree, which is what
made the clause do the deciding: on `xobject-image.pdf` `poppler` and `gs` draw nothing, `mupdf`
draws the page **black**, and we draw the one sample the file holds. Three readings, three
answers, none of them derivable from the others.

**The codestream's grid carries `MAX_SAMPLES` with it.** It is no longer the dictionary's grid —
which `decode_parts` bounds before any decoding — that decides what this allocates, and a frame
may state up to 65535 in each axis, which is sixteen times the bound. A frame stating zero in
either axis is refused as well; `zune-jpeg` answers that one at the header today, so the check
states the bound where the grid is chosen rather than being the only thing enforcing it.

## Consequences

`veraPDFHiResChangedHeight.pdf` now renders **pixel-identical** to the intact `veraPDFHiRes.pdf`
beside it — `compare -metric AE` is 0 over 581×295 — and its ink goes −94.953 → −0.195 against
the lightest reference, which is the *intact* file's own figure. `pdfCabinetOfHorrors`'s survey line is unchanged at two
documents reported — which is the point rather than a disappointment: the file is still reported,
and what changed is that its photograph is now on the page. A count that does not move is not
evidence that nothing happened.

The fixtures in `dct_components.rs` already built a hand-written baseline JPEG and a one-page PDF
around it, so the witness is pinned by a generated file rather than by the corpus: the dictionary
states 8×9, the frame states 8×8, and the test asserts the raster is 8×8 and the page complete.
That matters because the corpus witness lives in an optional submodule and no gate may depend on
one.

**What this round did not take, and why it is worth writing down.** `498264.pdf` is the only other
document in the chunk where a reference draws text and we draw none: its content stream's flate
data is corrupt 79 bytes in, after 18 bytes of output — `q\n30 31.16 552 729` — and `poppler`'s
inflate carries on past an "invalid distance too far back" to recover three lines of a heading.
`mupdf` and `gs` draw nothing, as we do, and no clause asks a reader to invent the rest of a
damaged stream. It is recorded in `doc/todo/03` as a *diagnosed* difference rather than a defect:
the question it raises is whether a truncated recovery is ever the right answer, and that is a
decision to make deliberately rather than by copying one renderer.
