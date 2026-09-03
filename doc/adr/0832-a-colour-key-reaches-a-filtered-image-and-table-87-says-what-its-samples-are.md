# ADR 0832 — A colour key reaches a filtered image: §8.9.6.4 warns about a lossy filter and excludes none, and Table 87 says what each filter's samples are

Status: accepted. Session 894.
Clauses: ISO 32000-2 §8.9.6.4 (colour key masking, and the sentence about `DCTDecode` and lossy
`JPXDecode`), §8.9.5.1 Table 87 (`/BitsPerComponent`, and what each filter delivers), §8.9.5.2
(the bit depth a component is decomposed into), §7.4.8 (`DCTDecode`), §11.6.4.3 (an `/SMask`
overriding a colour key).
Code: `crates/pdf-model/src/image.rs` (`colour_key_entry`'s refusal, `samples_of`'s `DCTDecode`
arm, `jpeg_colour_key`, `decode_ccitt` and `decode_jbig2`'s `Samples::colour_key`).
Tests: `crates/pdf-model/tests/dct_components.rs::a_colour_key_over_a_jpeg_hides_the_samples_it_names`,
`::a_colour_key_a_jpegs_samples_miss_leaves_the_image_painted`,
`::a_colour_key_masks_only_where_every_component_is_inside_its_range`;
`doc/checks/fixed-documents.toml`'s rows for `batch1/PDFBOX/PDFBOX-3631-15.pdf`,
`batch5/sumatrapdf/sumatrapdf-404-0.pdf` and `safedocs/cc-main-2021-31/1899/1899883.pdf`.
Measurement: `crates/pdf-model/examples/colour_key_mask_census.rs`.
Documents: §8.9.6, §8.9.6.4 and §7.4.8's ledger rows, `doc/todo/03` §44.

## Context

`doc/todo/03` §44 left `sumatrapdf-404-0.pdf` named: a bank statement whose 1800 × 600
`DeviceCMYK` banner is a `DCTDecode` stream carrying `/Mask [0 0 0 0 0 0 0 0]`, and which this
tree answered with

```
Image { name: "723529932_1800_600: colour-key /Mask on a DCTDecode image" }
```

That refusal was not this document's alone. `colour_key_entry` asked `image_codec` for the last
filter in the chain and refused **all four** codecs by name — `DCTDecode`, `JPXDecode`,
`JBIG2Decode`, `CCITTFaxDecode` — on a comment that gave two reasons: that the clause's NOTE 2
names the lossy pair, and that "a rule with three exceptions is worse than a rule".

## The clause

§8.9.6.4 states the requirement first, and it is addressed to whoever paints:

> Samples in the image that fall within this range shall not be painted, allowing the existing
> background to show through.

and the test, with its domain:

> For colour key masking, the value of the Mask entry shall be an array of 2 × 𝑛 integers,
> [min1max1 …min𝑛max𝑛] , where n is the number of colour components in the image's colour space.
> Each integer shall be in the range 0 to 2 BitsPerComponent - 1, representing colour values
> before decoding with the Decode array. An image sample shall be masked (not painted) if all of
> its colour components before decoding, c 1 … c n , fall within the specified ranges (that is,
> if min i ≤ c i ≤ max i for all 1 ≤ i ≤ n ).

and then, in its own paragraph, the sentence the refusal rested on:

> When colour key masking is specified, the use of a DCTDecode or lossy JPXDecode filter for the
> stream can produce unexpected results.

with NOTE 2 explaining that quantisation can move a sample out of the range its producer meant it
to fall in.

**`can` is not `shall not`.** That paragraph tells a *writer* choosing a filter what may go wrong
with the picture; it grants a reader nothing, states no exception to the `shall` above it, and is
followed by a NOTE, which is informative by ISO drafting convention. §7.4.8 was read for the same
purpose and says nothing about masking at all — its subject is the codestream's syntax, its
parameters and Table 13's `/ColorTransform`. So **refusing a lossy filter was this program's
choice wearing the clause's clothes**, which is the shape principle 5 exists to catch.

**Where the test is taken is stated too, and it is Table 87's `/BitsPerComponent` row**:

> If the image stream uses a filter, the value of BitsPerComponent shall be consistent with the
> size of the data samples that the filter delivers. In particular, a CCITTFaxDecode or
> JBIG2Decode filter shall always deliver 1-bit samples, a RunLengthDecode or DCTDecode filter
> shall always deliver 8-bit samples, and an LZWDecode or FlateDecode filter shall deliver
> samples of a specified size if a predictor function is used.

That closes the question the old comment had answered with an architecture rather than a clause.
"[B]efore decoding with the Decode array" means the samples the *filter* delivered, and the
standard says what those are for each of the three codecs concerned: one bit for the bilevel
pair, eight for `DCTDecode`. Nothing has to be un-decoded, and nothing is approximate — the
approximation NOTE 2 warns about is in the *encoder*, and it is the producer's.

## What is refused, and why that one is the standard's

**`JPXDecode` stays refused**, and the reason changes from the filter being lossy to the domain
being undefined. The same Table 87 row ends:

> If the image stream uses the JPXDecode filter, this entry is optional and shall be ignored if
> present. The bit depth is determined by the PDF processor in the process of decoding the JPEG
> 2000 image.

and §8.9.5.2 adds that for such an image the depth "is defined in the JPEG 2000 data and can have
different values per colour component". §8.9.6.4 bounds every one of its integers by
`2 BitsPerComponent - 1` — one entry, one bound, shared by every component. For a JPEG 2000 image
that entry shall be ignored, and the depth it would have named may differ from component to
component. So the ranges are stated in a domain the standard has taken away, and no reading of
§8.9.6.4 recovers one: this is a gap in what the *file* states, not in this reader, and it is
reported rather than guessed at. It is also the arm with no witness at all — see below.

## The population

`examples/colour_key_mask_census` counts image dictionaries whose `/Mask` is an array, by the
last filter in the chain, reading `pdf_syntax` alone rather than the code under test (trap 8).
Over **90 535 documents — `doc/pdf.js`'s 974, `doc/corpora`'s 275 and `corpus-cache`'s 89 286 —
of which 89 322 open**:

| last filter | images | documents |
|---|---|---|
| `FlateDecode` | 5794 | |
| `DCTDecode` | 68 | 17 |
| none | 24 | |
| `RunLengthDecode` | 17 | |
| `LZWDecode` | 17 | |
| `CCITTFaxDecode` | 13 | 4 |
| `ASCIIHexDecode` | 2 | |

**660 documents state one at all**, 5935 images in total, of which 67 are overridden by an
`/SMask` or a non-zero `/SMaskInData` (§11.6.4.3) and are not this clause's to apply. **Not one
image in the whole population is a `JPXDecode` or a `JBIG2Decode` one**, so the arm this ADR
keeps refusing has no witness and the arm it opens for the bilevel pair has four documents behind
it.

## What changed on the pages

Twenty-one documents state a colour key over a codestream on page one. Nineteen of them reported
it and now do not; the ink of each, by `tests/fixed_documents.rs`'s own instrument, before and
after:

- **`PDFBOX-3631-15.pdf` 7.9702 → 9.1754**, the largest move and the only one visible at a
  glance. It is SignRequest's tag-template demonstration: five `DCTDecode` stamps with
  `/Mask [254 255 254 255 254 255]` drawn over the page's own `[[s|0]]`, `[[d|0]]` and
  `[[t|0]]` placeholders. Painted opaque, each stamp is a white rectangle over the placeholder it
  is meant to sit beside; with the key applied the placeholders show through. **`pdftoppm
  -cropbox` and `mutool draw` both draw the second picture**, which is evidence for the reading
  and not the reason for it. `PDFBOX-3631-16.pdf` and `batch5/DSS/DSS-1356-8.pdf` are
  byte-identical copies of the same file.
- **`7926922.pdf` 19.8315 → 19.9751** and **`4359231.pdf` 13.2256 → 13.2507**, the next two.
- **`sumatrapdf-404-0.pdf` 17.3522 → 17.3508**, and this one is worth stating because it is the
  document the item named: 5384 of 386 019 pixels change, by at most 2 of 255. Its
  `/Mask [0 0 0 0 0 0 0 0]` asks for exactly-zero CMYK, and a lossy encoder left almost none of
  its white banner at exactly zero — **NOTE 2's own phenomenon, observed**. The refusal was
  costing the report rather than the picture here.
- The rest move by less than 0.02 or not at all, including all four `CCITTFaxDecode` documents:
  their ranges cover no sample the image has.

**Two documents keep a report and it is a different one.** `GHOSTSCRIPT-701468-0.pdf` and
`GHOSTSCRIPT-701474-1.pdf` write `/Mask [240 255]` on one-bit images, which
`colour_key_entry`'s existing check now reaches: `colour-key /Mask range 240..255 is outside 0..1
at 1 bits per component` — §8.9.6.4's own bound, and a sharper sentence than the filter's name.

## The construction

The bilevel pair cost one field apiece: `decode_ccitt` and `decode_jbig2` both build their raster
through `unpack`, which has applied the ranges since ADR 0023, and both were passing
`colour_key: None` with a comment pointing at the refusal.

`DCTDecode` needed the test placed. `zune-jpeg` hands over components, `convert_channels` turns
them into device RGB and applies §8.9.5.2's `/Decode` — so the values §8.9.6.4 ranges over exist
only between those two steps. `jpeg_colour_key` takes the answer there, as one flag per pixel,
and the flags are applied after the conversion. They cannot be applied before it: the conversion
writes the fourth byte of a four-component frame, where `k` lives until then, and would paint a
masked sample back in. The flags cost a quarter of what the raster beside them does, on a path
that runs only where a colour key is stated.

## Consequences

- §8.9.6.4 is `implemented` for every route but `JPXDecode`, whose refusal is now the standard's
  own silence rather than a judgement about lossiness.
- `unapplied_mask`'s contract holds: every reason it returns is still a reason `decode` will not
  have applied the mask.
- A page whose producer wrote a colour key over a lossy stream now shows what the encoder left
  behind, which can be a fringe of nearly-masked pixels. That is what the clause asks for and
  what NOTE 2 predicts; it is the producer's approximation and not this reader's.
