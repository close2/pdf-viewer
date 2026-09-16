# 1133 — A redaction decodes a codec image, clears the region, and re-encodes it FlateDecode

Status: accepted. Session 1136.
Context: `crates/pdf-transform/src/redact.rs` (the `Walk::plan_codec_clear`, `Walk::plan_image_clear`,
`ClearedImage`, `cleared_image_samples`, `build_cleared_image` items, and the `decoded` field on
`ImageClear`), `crates/pdf-transform/tests/redact.rs`, `crates/pdf-transform/Cargo.toml`.
Builds: ADR 1126 (round 1119's image-XObject sample destruction, whose `clear_region`, `row_stride`
and `ImageLayout` this reuses), ADR 1132 (round 1130's inline splice and its refusal set),
`pdf_model::image::decode`, `pdf_syntax::serialize::flate_encode`, `pdf_sandbox` isolation.
Clauses: ISO 32000-2 §12.5.6.23 (Table 195), §8.9.5, §8.9.5.2, §7.4.8, §7.4.6, §7.4.9, §11.6.4.2.

## 1. The codec-image case ADR 1126 refused is built for DCTDecode and CCITTFaxDecode

ADR 1126 destroyed the samples of a **codec-free** image XObject by zeroing its packed grid, and
refused any image behind a codec: "the samples are not re-encodable here without a codec writer".
A codec's stream bytes are the codec's *input*, not samples, so that packed-grid destruction has
nothing to address. This round decodes the image the interpreter's own way
(`pdf_model::image::decode` → straight-alpha `RGBA8` on the image's grid), clears the region in the
decoded raster with ADR 1126's `clear_region` (§8.9.5.2: sample 0 → `Dmin`, one constant), and
writes the opaque colour components as an 8-bit **`DeviceRGB` `FlateDecode`** image under a fresh
dictionary.

**Why FlateDecode, and why that keeps principle 1.** A lossy codec re-encoded lossily would leak:
the "cleared" region, run back through DCT, need not come out zero, and DCT block bleed can carry a
neighbour's original value into it. `FlateDecode` is lossless, so once the samples are zeroed they
stay exactly zero and the output image is no longer a codec at all. The redacted image cannot
round-trip back through the filter that would leak it. The cost is that the replacement is a device
raster rather than the source's colour space and depth — deliberate, and cheap: the census
(ADR 1126) finds no corpus `/Redact` meets any image.

**Why the codec runs where it does.** `pdf_model::image::decode` decodes under the process's
isolation — the confined worker in the program, in-process only where a caller set that (principle
3, trap 10). The redaction code hard-codes nothing; the test selects `Isolation::InProcess` so it
needs no worker binary, running the same `decode::here` routine the worker runs.

## 2. What stays refused, each with a narrower reason (trap 5, principle 1)

- **`JPXDecode`** — a codestream over the decoder's budget comes back at a reduced resolution level
  (§7.4.9 NOTE 3), so the raster is not the image's grid; a redaction that silently changed the
  image's resolution cannot be proven to have replaced the full-resolution content the region maps
  into. Refused rather than guess.
- **`JBIG2Decode`** — bilevel and lossless, so buildable by this same route, but not built and
  proven this round; an unproven clear is not shipped (decided per codec, an owed capability).
- A `DCTDecode`/`CCITTFaxDecode` image that **does not decode**, **decodes short of its grid** (the
  re-encode would drop rows), or **decodes with transparency** — a soft mask, a colour key, or an
  image mask leaves a non-opaque sample (§11.6.4.2) the opaque `DeviceRGB` re-encode cannot
  preserve — is refused by name rather than flattened.

The refusal is the whole image or nothing; a partly cleared image is never produced.

## 3. Census and proof

**Census (trap 8).** Rounds 1119/1130's parsed census stands: five corpus `/Redact`-bearing
documents (the Isartor 6.5.2 witness and four veraPDF PDF/A annotation fixtures), **none meeting any
image** — so none meets a codec image. The corpus submodules are not checked out in this worktree,
so this is the carried-forward parsed finding (the merge re-validates over the materialised tree).
The codec-image `shall` has no corpus witness and is proven by calibrated fixtures (trap 13), the
same standing as ADR 1126's XObject case and ADR 1132's inline case.

**Proof.** There is no JPEG or CCITT encoder in the tree, so the fixtures are captured bytes (PIL),
each decoded through the real codec and asserted against *that* decode — never a hand-predicted
pixel — so what they encode is self-checking; both are 16×16 with the halves distinct. After
redaction of the left half: the region pixels are the zero constant in the re-read image, the rest
byte-identical to the decode, the original codec bytes gone from the file, and the output a
`FlateDecode` `DeviceRGB` stream — and the file re-opens and the image decodes and draws.
`a_dct_image_in_the_region_is_decoded_cleared_and_reencoded_as_flate`,
`a_ccitt_image_in_the_region_is_decoded_cleared_and_reencoded_as_flate`, and
`an_image_behind_jpx_is_refused_by_name`. The row stays `partial`: a painted path or form, and the
refused codecs, are owed.
