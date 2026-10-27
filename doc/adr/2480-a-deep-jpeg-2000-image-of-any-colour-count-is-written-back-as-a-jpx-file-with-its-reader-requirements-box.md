# 2480 — A deep JPEG 2000 image of any colour-component count is written back as a JPX file with its Reader Requirements box

Session 1823, a "clause" round of batch one hundred and twenty-three, 2026-10-27.
Status: implemented — §12.5.6.23's `JPXDecode` refusal narrows to a component past twenty-four
bits; the row stays `departed` on ADR 1363's calculator function and on that remainder.
Context: ADR 2470 (the JP2 writer and the refusal this narrows), ADR 1371 (a codec's output is
the image's samples), ADR 2439 (the departures read), ADR 0187 (how the texts are cited), ADR 2481
(the sign statement, written by the same file).

## 1. The question

ADR 2470 wrote a deep cleared component back as a JP2 file and refused, past sixteen bits, an
image of other than one or three colour components, because ISO/IEC 15444-1 I.5.3.3 enumerates
greyscale and sRGB alone. §7.4.9 has the filter "expect to read a full JPX file structure", and
the container that holds four colour channels, or a colour specification a producer's data stated,
is the JPX file of ITU-T T.801 | ISO/IEC 15444-2 Annex M.

## 2. The decision

`pdf_jpx_write::file` takes a `Space` — JP2's greyscale and sRGB, enumerated 12 for CMYK, or a
`ColourBox` stated by the caller — and writes JP2 for the first two and JPX for the others;
`jp2` is `file` with no signs, and its output is byte for byte what it was.

- **Container.** The signature box, then a file type box with the brand `jpx ` and one
  compatibility entry naming it (M.8), then the Reader Requirements box (M.11.1) immediately after
  it, then the JP2 Header box and the codestream box. `jpxb`, the baseline brand, is not claimed:
  M.9.2.4's colour list has neither CMYK nor a vendor space.
- **Reader Requirements.** One mask length byte, a Fully Understand Aspects mask and a Decode
  Completely mask that are equal (a reader that supports every column reads the whole file), and
  one standard feature per column M.6.2.1 factors the requirements into: the codestream's coding
  (features 1 and 5), its placement (12, contiguous), opacity (8, 9 or 10) and, where T.801 Table M.14
  has one, the colour space. A stated colour box has no feature of its own and is left out of the
  expression; its colour specification box still states it. No vendor feature is listed.
- **Colour.** The redaction copies the first colour specification box the source file stated
  (`Headers.colour`, the order a conforming JP2 reader uses), METHDAT as the source wrote it:
  the four-byte space and its parameters for method 1, the profile for 2 and 3, the identifier
  and parameters for 4. A method T.801 Table M.22 reserves is refused by name, since its data cannot be
  shown to survive. An `APPROX` of 0, which a JP2 file sets and its readers ignore, is written 1,
  because M.11.7.2 does not admit 0 in a JPX file. A source with no box (a raw codestream) is written CMYK for four
  components and refused for any count other than one, three or four: §7.4.9 has such data state
  a vendor colour space, and the redaction invents no identifier, so a file that departs from
  §7.4.9's full JPX structure there is refused by name (the ledger row says so).
  The box is read, and these refusals made, only for an image that goes back as JPEG 2000, that
  is deeper than sixteen bits or states a sign; a shallow image goes back under `FlateDecode` as
  before and needs none.
  Images of one and three colour components keep the JP2 file ADR 2470 wrote: the dictionary's
  `/ColorSpace`, where it states one, governs by §7.4.9 in either container.
- **Opacity.** The writer can type the channel premultiplied (`cdef` type 2), but the tree's
  decoder (`hayro-jpeg2000`) reads only type 1 as opacity and rejects the box with type 2, so a
  file typed 2 fails to read back through `pdf_model::image::jpx_samples` (found by session 1825
  on its transcode fixture). The redaction therefore types its channel plain and leaves the
  premultiplication to Table 87's `/SMaskInData` 2, which it carries and which is what the image
  reader and a viewer read it from; the sandbox's decoder tolerates type 2 and so would not have
  shown it. `a_premultiplied_deep_jpx_image_reads_back_through_the_image_reader` plants this.
- **Proof.** The round trip through the tree's own decoder at four colour channels with and without
  an opacity channel, a vendor and an ICC box, and the redaction test of a four-component deep
  image read back sample for sample, through the sandbox's decoder and through the image reader; the `jpx_write` fuzz target draws CMYK and vendor spaces of
  one to four channels with signs.

## 3. What it costs

The reader requirements expression is derived from the features this one codestream shape needs,
not from the source file's own box, which a redaction does not carry across: the source's list
describes a codestream the redaction has re-encoded. A viewer that treats an unrecognised
vendor colour as unreadable will refuse the output where it would have refused the source.
