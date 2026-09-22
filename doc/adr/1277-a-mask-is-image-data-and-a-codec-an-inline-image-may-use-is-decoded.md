# 1277 — A mask is image data, an opacity channel becomes the soft mask the table names, and a codec an inline image may use is decoded

Status: **accepted**.
Context: `crates/pdf-transform/src/redact.rs` (`Walk::plan_image_clear`, `Walk::stated_masks`,
`Walk::plan_mask_clear`, `Walk::plan_packed_clear`, `Walk::plan_codec_clear`,
`Walk::jpx_admits_a_clear`, `reexpressed`, `stencil_samples`, `cleared_image_samples`,
`build_cleared_image`, `raster_dictionary`, `Walk::spliced_inline_image`,
`Walk::inline_codec_raster`, `build_inline_image`, `colour_space_resource`),
`crates/pdf-transform/tests/redact.rs`.
Builds: ADR 1126 (the zero constant), ADR 1133 (the codec re-encode), ADR 1143 (the bilevel one),
ADR 1196 (copy a shared object rather than replace it), ADR 1248 (the JPEG 2000 conditions).
Amends: ADR 1133's and ADR 1248's refusal of a codec image "carrying transparency" and of a non-zero
`/SMaskInData`, and ADR 1126's refusal of an inline image behind a codec or naming a colour-space
resource. Each is read against the clause, not dropped.
Clauses: ISO 32000-2 §12.5.6.23, §8.9.5.1 (Table 87), §8.9.6.2, §8.9.6.3, §8.9.6.4, §8.9.7,
§11.6.5.2 (Table 143), §11.6.4.3, §7.4.9.

## 1. A mask is image data, and it was being left behind

§12.5.6.23 asks that "that portion of the image data shall be destroyed". A picture's `/SMask`
(§11.6.5.2) and `/Mask` stream (§8.9.6.3) are images of their own: each holds, sample by sample, the
shape of what the picture draws. The codec-free clear carried both entries by reference, untouched —
so the silhouette of the removed content survived in the mask while the picture's samples were
zeroed. That was a trace the refusal list did not name. Now each mask is **cleared as an image of its
own**, on its own grid, under the picture's placement: Table 143 puts a soft mask on the picture's
unit square "regardless of whether the samples coincide individually", and §8.9.6.3 says the two
boundaries "will coincide". The same sharing rule as ADR 1196 decides replace or copy, with one
addition: a mask is copied wherever its picture is.

## 2. A codec picture is decoded with its masks set aside

ADR 1133's refusal of a codec image "carrying transparency" followed from decoding the picture *with*
its masks multiplied in, then having nowhere to put the alpha. Decoding it without them — a copy of
the dictionary with `/SMask` and `/Mask` removed — gives the picture's own samples, and the masks are
cleared by §1 and named again by the fresh dictionary. Still refused, each with its sentence: a
colour-key `/Mask` on a codec picture (§8.9.6.4's ranges are "colour values before decoding with the
Decode array" in a sample domain the `DeviceRGB` re-encode leaves), and a soft mask stating
§11.6.5.2's `/Matte` under a codec picture (the pre-blending is in the picture's own colour space).

## 3. `/SMaskInData` names what to create

Table 87's codes 1 and 2 each say "[a] PDF processor shall create a soft-mask image from the
information" (code 2 from "the opacity channel information"). ADR 1248 refused both because the
opaque re-encode had no channel. The clause names the channel: the decode's alpha, with code 2's
premultiplication already undone by the decoder, is written as an 8-bit `DeviceGray` soft-mask image
on the picture's grid, cleared under the same placements. §11.6.4.3 has that embedded mask override
`/Mask`, and Table 87 says `/SMask` "shall not be specified", so neither entry is carried. A code
Table 87 does not define is refused. Over eight bits a component stays refused: the decoder
delivers eight.

## 4. The kind of raster is read off the decode, not assumed

Four fresh rasters now exist (`RasterKind`), and each is admitted only where the decode's output fits
it: an opaque kind needs every pixel opaque; a soft mask behind a codec needs its three channels equal;
a stencil (a §8.9.6.2 image mask behind a codec) needs every pixel exactly painted or unpainted. That
is how ADR 1236 decided the stroke: from the output.

## 5. An inline image's codec and its resource name

§8.9.7 leaves two codecs to an inline image — "JBIG2Decode , Crypt and JPXDecode are not listed …
because those filters shall not be used with inline images" — so `DCTDecode` and `CCITTFaxDecode`
are decoded and re-expressed as an image `XObject` behind them is, and a forbidden filter is refused
by name. A `/CS` name the scan resolved into a resource is written back as that name: §8.9.7 says
"the value of the ColorSpace entry may also be the name of a colour space in the ColorSpace
subdictionary of the current resource dictionary", and the resources in force at the splice are
the ones in force at the `BI`.

## What is left, and why

A reduced-resolution `JPXDecode` decode (§7.4.9 NOTE 3) is the decoder's budget, a principle-3 bound
this verb does not raise; a codestream deeper than eight bits needs a decoder that delivers its
depth. A stroke whose outline holds an arc (ADR 1236) is untouched here. A mask behind a codec is
cleared where its decode fits a kind, refused otherwise.
