# 1143 — a redaction clears a JBIG2 bilevel image at its own one bit

2026-09-16. ADR 1143, continuing 1136 (ADR 1133). Contract: build the `JBIG2Decode` case 1136 left
owed — the same decode → clear → re-encode §12.5.6.23 asks for — and decide the bilevel re-encode.

**The build** (`crates/pdf-transform/src/redact.rs`). `plan_codec_clear` now accepts `JBIG2Decode`
(§7.4.7): it decodes the image the interpreter's own way (`pdf_model::image::decode` → opaque
`RGBA8`, confined — principle 3), checks full opacity (§11.6.4.2), then branches on codec family. A
colour codec → `colour_samples` (8-bit `DeviceRGB`, unchanged); JBIG2 → `bilevel_samples`, which
packs the decode to **1-bit `DeviceGray`** MSB-first per row (§8.9.5.2). `clear_region` zeroes the
region's bits; §8.9.5.2 maps sample 0 to Dmin, so the zero constant is one bit of black.
`build_cleared_image` writes a fresh `DeviceGray`/1-bit dictionary, the `ClearedImage.rgb` marker
generalised to `codec: Option<ImageLayout>`.

**The decision (ADR 1143).** A bilevel image is re-expressed at its own depth, not inflated to
eight bits: three 8-bit components for two levels would misdescribe it and cost 24× the bytes.

**A latent leak, found and fixed (trap 13, principle 1).** `carry` restated the page's image
reference by `copy_closure` on it, which walked the **replaced** image's *source* dictionary — and a
JBIG2 image's `/DecodeParms` references a separate `/JBIG2Globals` symbol-dictionary stream, copied
into the output as an orphan holding the original bilevel bytes. Colour codecs hid this (inline
`/DecodeParms`). `carry` now maps an already-placed reference to its slot without walking its
source, so the globals is reached from nowhere and never written; the test asserts both gone.

**Refused, unchanged (trap 5).** `JPXDecode` (reduced-resolution decode, §7.4.9 NOTE 3); a painted
path or form (no per-region unit, ADR 1126 §3); a JBIG2 image that does not decode, decodes short of
its grid, or decodes with transparency.

**Proven unrecoverable.** No JBIG2 encoder exists in the tree, so the fixture is captured bytes: the
ISO 32000-2 §7.4.7 worked example (a 52×66 bilevel image, a letter C drawn twice — the same
bitstream `pdf_sandbox`'s §7.4.7 test decodes, principle 5), decoded through the real codec and
asserted against that decode. After redacting the left half:
`a_jbig2_image_in_the_region_is_decoded_cleared_and_reencoded_as_flate` proves the region pixels are
the one-bit zero constant in the re-read image, the rest byte-identical to the decode, the JBIG2
image and its orphaned globals gone, the output a `FlateDecode` 1-bit `DeviceGray` stream that
re-opens, decodes and draws.

**Row moved:** §12.5.6.23 stays `partial` (the codec-image `shall` now met for DCT/CCITT/JBIG2;
overlay departs; path/form and JPX owed), note rewritten, cites ADR 1143. All 16 `redact` tests
pass. Scoped to `redact*`, `carry`, the row, the ADR. Exact gate lines and exit codes in the report.
