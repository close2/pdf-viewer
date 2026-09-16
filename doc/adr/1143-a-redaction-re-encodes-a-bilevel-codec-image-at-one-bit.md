# 1143 — A redaction re-encodes a bilevel codec image at its own one bit, DeviceGray

Status: accepted. Session 1143.
Context: `crates/pdf-transform/src/redact.rs` (the `Walk::plan_codec_clear`, `bilevel_samples`,
`colour_samples`, `build_cleared_image`, `cleared_image_samples` items, the `codec` field on
`ClearedImage`, and the `carry` short-circuit), `crates/pdf-transform/tests/redact.rs`.
Builds: ADR 1133 (round 1136's colour-codec clear, whose decode-clear-Flate route and refusal set
this reuses), ADR 1126, `pdf_model::image::decode`, `pdf_sandbox` JBIG2 isolation.
Clauses: ISO 32000-2 §12.5.6.23 (Table 195), §7.4.7, §8.9.5, §8.9.5.2, §11.6.4.2.

## 1. JBIG2Decode is the codec case ADR 1133 owed, built at one bit rather than eight

ADR 1133 decoded a `DCTDecode`/`CCITTFaxDecode` image, cleared the region, and re-encoded it as an
opaque 8-bit `DeviceRGB` `FlateDecode` raster, and refused `JBIG2Decode` as "buildable by this same
route but not built this round". This round decodes it the same way (`pdf_model::image::decode` →
straight-alpha `RGBA8` under the process's isolation, principle 3), clears the region with the same
`clear_region`, and re-encodes it `FlateDecode` — lossless, so the cleared region stays exactly the
zero constant and cannot round-trip back through the codec.

**The decision beyond ADR 1133 is the raster's shape.** `JBIG2Decode` (§7.4.7) is a *bilevel*
image: Table 87 makes it "always deliver 1-bit samples". ADR 1133's 8-bit `DeviceRGB` was for
colour codecs; re-expressing a one-bit image as three eight-bit components would inflate it 24-fold
and misdescribe what it is. So the bilevel clear is written as a **1-bit `DeviceGray`
`FlateDecode`** image, at the source's own depth. `bilevel_samples` packs the decode to one bit per
pixel MSB-first per row (§8.9.5.2): a white pixel is sample 1 (Dmax), a black pixel sample 0 (Dmin),
which is the `DeviceGray` sense of the bits the filter delivers.

**The zero constant is one bit.** §8.9.5.2 maps sample 0 through the default `/Decode` to Dmin, so
`zero_sample` clearing one bit yields black — the same result the colour route reaches by zeroing
three bytes, at a fraction of the size. The replacement is opaque and carries no `/Decode`, `/Mask`
or `/SMask`, so a JBIG2 image that decodes with transparency (an image mask, a soft mask, a colour
key) is refused rather than flattened, exactly as the colour route refuses it (§11.6.4.2).

## 2. A replaced image's original dictionary must not be walked back in

`carry` restates a built page's references in the output's numbering by calling `copy_closure` on
each. A **replaced** object (a reserved image slot) has no original in the output, but `copy_closure`
started at its id walked the *source* image dictionary — and a JBIG2 image's dictionary references a
separate `/JBIG2Globals` symbol-dictionary stream. That orphan was copied into the output, leaving
the original bilevel image's bytes recoverable (principle 1). The colour codecs hid this: their
`/DecodeParms` are inline, so nothing separate was reached. `carry` now maps an already-placed
reference to its slot without walking its source value, so the globals stream is reached from
nowhere and never written — the destruction is a destruction.

## 3. What stays refused, and the row

`JPXDecode` stays refused (ADR 1133 §2: an over-budget codestream returns a reduced resolution
level, §7.4.9 NOTE 3, so its raster is not the image's grid). A painted path or form stays refused
(ADR 1126 §3: no per-region unit without geometric subtraction). §12.5.6.23 stays `partial` on
those, honestly: the codec-image `shall` is now met for every codec this build decodes.

## 4. Census and proof

Census (trap 8): rounds 1119/1130's parsed census stands — five corpus `/Redact` documents, none
meeting any image, so none a codec image; the codec-image `shall` has no corpus witness and is
proven by fixtures, as ADR 1126's and 1133's are. There is no JBIG2 encoder in the tree, so the
fixture is captured bytes: the ISO 32000-2 §7.4.7 worked example (a 52×66 bilevel image, a letter C
drawn twice — the same bitstream `pdf_sandbox`'s own §7.4.7 test decodes, never another
implementation's output, principle 5), decoded through the real codec and asserted against *that*
decode. After redaction of the left half:
`a_jbig2_image_in_the_region_is_decoded_cleared_and_reencoded_as_flate` proves the region pixels are
the one-bit zero constant in the re-read image, the rest byte-identical to the decode, the original
JBIG2 image and its orphaned globals gone from the file, and the output a `FlateDecode` 1-bit
`DeviceGray` stream that re-opens, decodes and draws.
