# 505 — The row a photograph was refused for

**Finding.** `doc/todo/03` §7 says a corpora round wanting a finding should point the ink
assertion at `pdfCabinetOfHorrors` or `govdocs1-error-pdfs`, "where the files do *not* share a
page and the method therefore needs a reference again". Both were taken — **78 documents** —
against `pdftoppm`, `mutool` and `gs` at 72 dpi, every one told to use the crop box, ranked by
**our ink minus the lightest live reference's**. One row of 78 was not a rasterisation
difference: `veraPDFHiResChangedHeight.pdf` at **−94.953**, which is the whole page. It is
`veraPDFHiRes.pdf` with one digit of its `/Height` altered — 1226 → 1227 — and `decode_jpeg`
refused the image outright for it, on a comment whose premise had expired. **§7.4.8 states no
match requirement at all**; it puts a JPEG's dimensions in the encoded data and says the decoder
may read them from there, while §7.4.9 states the requirement for JPEG 2000 in so many words. So
a `DCTDecode` image is now built on the grid its codestream states, and the file renders
**pixel-identical** to the intact one beside it. The oracle then decided the second half within
the round: silent, `xobject-image.pdf` — 200×100 described, **one red pixel** held — entered the
judged set for the first time and was contradicted at 99.78, so the contradiction is now reported
*beside* the drawing.

**Date.** 2026-08-14.
**ADR.** [0340](../adr/0340-the-row-a-photograph-was-refused-for.md).
**Touched.** `crates/pdf-model/src/image.rs` (`DecodedJpeg`, `decode_jpeg`'s reading and its
bound, `contradicted_frame`, three comments that said "for every codec but one"),
`crates/pdf-model/src/content/image.rs` (the report beside the drawing),
`crates/pdf-model/tests/dct_components.rs` (the module's subject widened to both halves of
§7.4.8's sentence, `pdf_with_image` takes the stated grid, `interpret_one`, two new tests),
`doc/conformance/ledger.toml` (§7.4.8's row and its test list),
`doc/todo/03-more-corpora.md` (the chunk recorded, §7 amended),
`doc/adr/0340-*` (new), this file.

## The chunk, and what it says

| | documents | opened | refused loudly | reported |
|---|---|---|---|---|
| `pdfCabinetOfHorrors` | 24 | 23 | 1 (`encryption_openpassword.pdf`, no password) | 2 |
| `govdocs1-error-pdfs` | 54 | 54 | 0 | 6 |

Both survey lines reproduce the four-hundred-and-sixty-seventh session's exactly, before and
after this round's fix — which is the point rather than a disappointment, and is the entry
`doc/todo/02` §7 keeps for it: *a count that improves is not a picture, and a count that does not
move is not evidence that nothing happened.* `veraPDFHiResChangedHeight.pdf` is reported before
and after. What changed is that its photograph is now on the page.

**Nothing failed to open for a reason that is this tree's, for the fifth population running.**
The one refusal is a document that wants a password to open at all.

## The ranking, read from the bottom

Every negative row past −0.2, with what it is:

| delta | document | what it is |
|---|---|---|
| **−94.953 → −0.195** | `veraPDFHiResChangedHeight` | **this round's defect**, and now the intact file's own figure |
| −1.719 | `507676` | an inline image whose `/DP` runs off the end and a content stream full of binary read as operators — eleven reports |
| −1.178 | `435321` | a `head` table truncated at 2048 bytes of 10274; `gs` draws **nothing** at all for it |
| −0.802 | `498264` | a flate content stream corrupt 79 bytes in; `mupdf` and `gs` draw nothing either — see below |
| −0.643 … −0.20 | fourteen documents | glyph rasterisation weight. The two largest, `032270` and `427330`, were read side by side against `poppler` and `mupdf` at full size: the same page three times |
| −0.481, −0.395 | `020747`, `477139` | a CFF program that would not parse — `doc/todo/21`'s population |

And the positive tail is where the references disagree with *each other*: `178360` is ours 40.7,
`poppler` 26.2, `mupdf` 110.7, `gs` 50.7 on a page whose eight fonts we report; `073439` and
`509284` put us within 0.07 of `poppler` while `mupdf` draws 40% less ink.
`digitally_signed_3D_Portfolio` is +13.3 only because `gs` renders a different page box —
612×792 where the other three agree on 504×360, which is trap 3 arriving as a reading rather
than as a misconfiguration, since the invocation is explicit.

## `498264.pdf`, diagnosed and not taken

The one other document in the chunk where a reference draws text and we draw none. Its page-one
content stream inflates **18 bytes** — `q\n30 31.16 552 729` — and then fails with zlib's
"invalid distance too far back" after consuming 79 of its 2649 input bytes. `poppler` recovers
three lines of a heading past that point; `mupdf` and `ghostscript` draw nothing, as we do, and
this tree reports `Undecodable`. No clause asks a reader to invent the rest of a damaged stream,
so it is recorded in `doc/todo/03` as a question — *is a truncated recovery ever the right
answer?* — rather than fixed by copying the one renderer that does it.

## What the fix cost and what it bought

The corpus gate's incomplete count is **unchanged**, and the two documents that write this
construct are the reason: `xobject-image.pdf` and `issue6413.pdf` were reported before, as
refusals, and are reported now, as contradictions that were drawn anyway. Measured with the
change stashed and unstashed, both runs print the same figure. On `issue6413.pdf` — dictionary
1×1, frame 213×5 — our ink now sits at 6.318 against `poppler`'s 6.376 and `gs`'s 6.390, where
`mupdf` is the outlier at 3.550.

The oracle is green with no new contradiction, and `xobject-image.pdf` is out of the judged set
for the honest reason: a page whose file contradicts itself about its own image is not a page we
can claim to agree about.
