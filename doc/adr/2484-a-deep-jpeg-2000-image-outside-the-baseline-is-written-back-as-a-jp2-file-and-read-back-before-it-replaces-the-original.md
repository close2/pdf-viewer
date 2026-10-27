# 2484 — A deep JPEG 2000 image outside the baseline is written back as a JP2 file, and read back before it replaces the original

Session 1825, an "archive" round of batch one hundred and twenty-three, 2026-10-27.
Status: implemented — the transcode's refusal of a component past sixteen bits is ended for one
colour channel or three, with an opacity channel of the colour channels' depth beside three; the
census finds no corpus document it frees, so the converted count does not move.
Context: ADR 1383 (the JPX baseline), ADR 1400 (the stated `/ColorSpace` rule), ADR 1371 (a codec's
output is the image's samples), ADR 1412 (the opacity channel), ADR 2470 (the lossless JP2 writer),
ADR 2485 (the two refusals veraPDF's tests forced).

## 1. The question

`preserve` for a JPEG 2000 image outside the JPX baseline writes its samples again under
`FlateDecode` in the colour space the dictionary states (ADR 1400). Table 87 bounds every filter
but `JPXDecode` at sixteen bits a sample, while §7.4.9 lets a component be as deep as 38, so a
Flate copy has no field for a component past sixteen and the transcode named that as
`DEEPER_THAN_SIXTEEN`. ADR 2470 gave the tree a lossless writer (`pdf_jpx_write`); this ADR uses it
at the transcode's site.

## 2. The construction

`transcode` sends an image whose precision is past sixteen to `written_as_jpeg_2000`, which:

- writes the samples through `pdf_jpx_write::jp2`, greyscale for one colour channel and sRGB for
  three. A JP2 file is a plain one-tile codestream inside ISO/IEC 15444-1 Annex I's boxes, which is
  inside T.801 M.9.2's baseline (M.9.2.3 asks the codestream for no extension; M.9.2.4's list holds
  the enumerated space of the file's one specification). ISO 19005-2 section 6.2.8.3 and ISO
  19005-4 section 6.2.7.3 then bind the new data and admit it: one, three or four colour channels,
  a depth from 1 to 38, one depth for every colour channel.
- writes the colour planes at the widest colour depth. Where the components had different depths,
  `/Decode` is widened by `redact::widened_decode` from each component's own depth so §8.9.5.2's
  map gives every integer the value it had (the redaction writer's construction, ADR 1371).
- keeps the dictionary the producer wrote: `JPXDecode`, the stated `/ColorSpace` that §7.4.9 has
  govern the file's own specifications, and `/SMaskInData`. An opacity channel travels inside the
  file, so no soft-mask image is created (the Flate route's construction in ADR 1412 is not
  needed). The channel is written as the plain opacity of channel type 1 whatever `/SMaskInData`
  says, because the decoder this tree reads with takes only type 1 as an opacity channel; the
  premultiplication stays the dictionary's statement.
- **proves the file before it replaces the original.** `reads_back_as` decodes the written file
  through `pdf_model::image::jpx_samples` and requires the grid, the component count, the
  precision, every colour integer, the opacity integers and depth, and the premultiplication to
  equal what was written from. A mismatch is the existing `NOT_PROVED` refusal; a writer failure is
  `NOT_WRITTEN_AS_JPEG_2000`.

`TranscodedImage` gained its `filter` so the report's row says `JPXDecode` for this route and not
`FlateDecode`.

**Named refusals that remain**, each a `Because::NotBuiltYet` with its sentence: four colour
channels (`DEEP_WITHOUT_A_JP2_SPACE`, waiting for the JPX file ADR 2480 writes), and the two ADR
2485 argues, an opacity channel of another depth (`DEEP_OPACITY_AT_ANOTHER_DEPTH`) and a grey image
with an opacity channel (`DEEP_GREY_WITH_OPACITY`).

## 3. The rewrite's public id

The `Rewrite` variant keeps its id `jpeg2000-transcoded-to-flate` although a deep image now leaves
as JPEG 2000. The id is public: configuration files, reports and the documents that name it already
spell it, and renaming it breaks them for a word. **The cost** is a name that is wrong for one route;
the describe text and the variant's doc comment state that a component past sixteen bits is written
as JPEG 2000 again, and the report's row carries the filter actually written. A rename belongs to
an owner decision about the public vocabulary, not to this remedy.

## 4. The census, and why the remedy stands

The ranking's scan (a scratch example, not committed) opened every image a JPX-site finding names
and read its precision: pdf.js 3 images (all 8-bit, a stated `/ColorSpace`; 963 of 974 documents
opened), `doc/corpora` 0 (487 of 503 opened), the veraPDF corpus 4 (all 8-bit, no `/ColorSpace`
stated; 2908 of 2908 opened). **No image in any corpus has a component deeper than sixteen bits**,
so no document is freed and the archive census's converted count is unchanged, with and without the
CJK pack. The three pdf.js images are refused earlier, at the shipped profiles' default decision
(they do not `preserve` the site), so the refusal this ADR ends was never what held them.

The remedy stands for two reasons. The refusal was a statement that the tree *could not* write such a
file, and ADR 2470 made that untrue; §7.4.9 and ISO 19005 both admit the depths, so a document that
carries one converts or is refused for a reason that is true. And the corpus is a sample of
producers, not of what exists (the robustness denominator, CLAUDE.md principle 5): a document no
corpus holds can carry the depths §7.4.9 admits.

## 5. The proof

Seven tests in `tests/archive/deep_jpx.rs` plant documents, convert them under both targets and read
the output back: three colour channels at one depth, mixed depths (20, 18, 24) with the widened
`/Decode`, opacity of the colour depth (premultiplied code 1 and code 2), a grey image with a
non-default `/Decode`, and the three named refusals; a unit test in `transcode.rs` reads a channel
out of its field. Planting the defect the tests catch: disabling the route fails all seven; disabling the `/Decode` widening fails the mixed-depth test. veraPDF
1.30.3 passed 12 of 12 planted outputs, at ISO 19005-2 2b and ISO 19005-4.

## 6. Found, not fixed

- Drawing a deep image with a widened non-default `/Decode` through the `pdf-model` reader
  posterises (the mixed-depth image drew at about four bits of red against its source; a uniform
  depth draws identically). The written integers and `/Decode` pairs are §8.9.5.2's exact map, so
  the fault is in the reader, probably the 16-bit cap of the `Decode` table in `image/space.rs`;
  unconfirmed, owned by whoever takes the image reader.
- The decoder reads a channel of type 2 (premultiplied opacity, ISO/IEC 15444-1 I.5.3.6) as no
  opacity, so a file the redaction writer writes with that type does not read back through
  `pdf-model`; the redaction writer's own tests should check it.
